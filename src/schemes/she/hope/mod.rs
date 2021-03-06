//! This is the documentation for the `AhOPE` scheme:
//!
//! * Developped by
//! * Published in
//! * Available from
//! * Type encryption (order preserving)
//! * Setting bilinear groups (asymmetric)
//! * Authors Georg Bramm
//! * Date: 04/2018
//!
//! # Examples
//!
//! ```
//!use rabe::schemes::hOPE::*;
//!let (_pk, _msk) = setup();
//! ```
extern crate bn;
extern crate rand;
 extern crate serde;
#[cfg(feature = "serde_derive")] 
extern crate serde_derive;
extern crate paillier;
extern crate serde_json;

pub mod bplus;
pub mod websocket;
use crate::schemes::she::hope::bplus::tree::Node;
use crate::schemes::she::hope::bplus::tree::node::Leaf;
use crate::schemes::she::hope::bplus::Tree;
use bincode::serialize;
use bn::*;
use std::string::String;
use paillier::*;
use paillier::Sub;
use std::cmp::Ordering;
use std::ops::AddAssign;
use std::ops::Sub as StdSub;
use std::ops::SubAssign;
use std::ops::Mul;
use std::fs::File;
use std::error::Error;
use std::io::{Read, Write};
use std::path::Path;
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use mongodb::oid::ObjectId;
use serde_derive::{Serialize, Deserialize};

/// A hOPE SYSTEM (SP)
#[derive(Serialize, Deserialize, Clone)]
pub struct hope {
    pub _id: ObjectId,
    // mongodb collection name
    pub _name: String,
    // p element of hOPE scheme
    pub _p: bn::G1,
    // p element of hOPE scheme
    pub _q: bn::G2,
    // code tree of hOPE scheme
    pub _tree: Tree,
    // lookup table of hOPE scheme
    pub _apl: BTreeMap<Vec<u8>, ObjectId>,
    // keypair
    pub _key: Option<hopeKey>,
}

//impl Actor for System {
//    type Context = ws::WebsocketContext<Self>;
//

/// A hOPE APL TABLE (APL)
#[derive(Serialize, Deserialize, Clone)]
pub struct hopeCiphertext {
    pub _id: ObjectId,
    pub _c: EncodedCiphertext<u64>,
    pub _g: bn::G1,
    pub _h: bn::Gt,
    pub _o: u64,
}

/// A hOPE KEY (SK)
#[derive(Serialize, Deserialize, Clone)]
pub struct hopeKey {
    pub _dk: Option<DecryptionKey>,
    pub _ek: EncryptionKey,
}

impl hopeKey {
    pub fn new() -> hopeKey {
        // generate a fresh keypair and extract encryption and decryption key
        let (ek, dk) = Paillier::keypair().keys();
        // return hopeKey
        hopeKey {
            _dk: Some(dk),
            _ek: ek,
        }
    }
}

impl hope {
    pub fn new(_name: String, _degree: usize) -> hope {
        // random number generator
        let _rng = &mut rand::thread_rng();
        // return System
        hope {
            _id: ObjectId::new().unwrap(),
            _name: _name,
            _p: G1::random(_rng),
            _q: G2::random(_rng),
            _tree: Tree::new(_degree),
            _apl: BTreeMap::new(),
            _key: hope::keygen(),
        }
    }

    pub fn keygen() -> Option<hopeKey> {
        Some(hopeKey::new())
    }

    pub fn encrypt(&mut self, _m: u64) -> Option<hopeCiphertext> {
        if let Some(_ek) = self.enc_key() {
            self.encrypt_ek(&_ek, _m);
        }
        None
    }

    pub fn encrypt_ek(&mut self, _ek: &EncryptionKey, _m: u64) -> Option<hopeCiphertext> {
        let _c = Paillier::encrypt(_ek, _m);
        // return pk_u and sk_u
        match Fr::from_str(&_m.to_string()) {
            Some(_fr) => {
                let _g = self._p.mul(_fr);
                match self.lookup_apl(_g) {
                    Some(_ct) => return Some(_ct),
                    None => {
                        let _h = pairing(_g, self._q);
                        let _id = ObjectId::new().unwrap();
                        let leaf = Leaf::new(_id.clone(), _c.clone());
                        self.insert_tree(leaf);
                        self.update_tree();
                        match self.lookup_tree(_id.clone()) {
                            Some(_code) => {
                                let _hct = hopeCiphertext::from_id(_id, _c, _g, _h, _code);
                                match self.insert_apl(_hct.clone()) {
                                    Some(ins_res) => Some(_hct),
                                    None => None,
                                }
                            }
                            None => None,
                        }
                    }
                }
            }
            None => None,
        }
    }

    pub fn decrypt(&self, _ct: hopeCiphertext, _dk: DecryptionKey) -> u64 {
        Paillier::decrypt(&_dk, &_ct._c)
    }

    pub fn add(&mut self, _ct1: &hopeCiphertext, _ct2: &hopeCiphertext) -> Option<hopeCiphertext> {
        if let Some(ek) = self.enc_key() {
            let _g1 = _ct1._g + _ct2._g;
            match self.lookup_apl(_g1) {
                Some(_ct) => return Some(_ct),
                None => {
                    let _h1 = pairing(_g1, self._q);
                    let _c = Paillier::add(&ek, &_ct1._c, &_ct2._c);
                    let _id = ObjectId::new().unwrap();
                    Paillier::rerandomize(&ek, &_c);
                    let leaf = Leaf::new(_id.clone(), _c.clone());
                    self.insert_tree(leaf);
                    self.update_tree();
                    match self.lookup_tree(_id.clone()) {
                        Some(_code) => {
                            let _hct = hopeCiphertext::from_id(_id, _c, _g1, _h1, _code);
                            match self.insert_apl(_hct.clone()) {
                                Some(ins_res) => return Some(_hct),
                                None => return None,
                            }
                        }
                        None => return None,
                    }
                }
            }
        }
        None
    }

    pub fn sub(&mut self, _ct1: &hopeCiphertext, _ct2: &hopeCiphertext) -> Option<hopeCiphertext> {
        if let Some(ek) = self.enc_key() {
            let _g1 = _ct1._g - _ct2._g;
            match self.lookup_apl(_g1) {
                Some(_ct) => return Some(_ct),
                None => {
                    let _id = ObjectId::new().unwrap();
                    let _c = Paillier::sub(&ek, &_ct1._c, &_ct2._c);
                    Paillier::rerandomize(&ek, &_c);
                    let leaf = Leaf::new(_id.clone(), _c.clone());
                    self.insert_tree(leaf);
                    match self.lookup_tree(_id.clone()) {
                        Some(_code) => {
                            let _h1 = pairing(_g1, self._q);
                            let _hct = hopeCiphertext::from_id(_id, _c, _g1, _h1, _code);
                            match self.insert_apl(_hct.clone()) {
                                Some(ins_res) => return Some(_hct),
                                None => return None,
                            }
                        }
                        None => return None,
                    }
                }
            }
        }
        None
    }


    //pub fn ask_client<T>(_req: &ProtocolReq<T>, ctx: &mut Self::Context) -> ProtocolRes<T> {
    //Paillier::decrypt(_pk._key, _ct._c);
    //}


    pub fn fetch_ct(&self, _id: ObjectId) -> Option<hopeCiphertext> {
        None
    }

    pub fn insert_ct(&self, _ct: hopeCiphertext) -> Option<ObjectId> {
        None
    }

    pub fn insert_tree(&mut self, _elem: Leaf) {
        self._tree.insert(_elem);
    }

    pub fn update_tree(&self) {
        //self._tree.update_apl(&MONGO.collection(&self._coll))
    }

    pub fn lookup_tree(&self, _id: ObjectId) -> Option<u64> {
        self._tree.code(_id)
    }

    pub fn lookup_apl(&self, _token: bn::G1) -> Option<hopeCiphertext> {
        match serialize(&_token) {
            Err(_) => return None,
            Ok(_g) => {
                match self._apl.get(&_g) {
                    Some(_id) => return self.fetch_ct(_id.clone()),
                    None => return None,
                }
            }
        }
    }

    pub fn insert_apl(&mut self, _elem: hopeCiphertext) -> Option<ObjectId> {
        match serialize(&_elem._g) {
            Err(_) => return None,
            Ok(_g) => self._apl.insert(_g, _elem._id),
        }
    }

    //pub fn lookup_ppl(&self, _token: Document) -> Option<hopeCiphertext> {}
    // omitted

    //pub fn insert_ppl(&self, _elem: hopeCiphertext) -> Option<InsertOneResult> {}
    // omitted

    pub fn keys(&self) -> Option<hopeKey> {
        return self._key.clone();
    }

    pub fn enc_key(&self) -> Option<EncryptionKey> {
        if let Some(ref _k) = &self._key {
            return Some(_k._ek.clone());
        }
        None
    }

    pub fn dec_key(&self) -> Option<DecryptionKey> {
        if let Some(ref _k) = &self._key {
            return _k._dk.clone();
        }
        None
    }
}
/*


impl AddAssign for hopeCiphertext<T> {
    fn add_assign(&mut self, other: Self) {
        let _g1 = self._g.add(other._g);
        match lookup_apl(_g1) {
            Some(_ct) => {
                return _ct;
            }
            None => {
                let _c = Paillier::add(&_pk._key, self._c, other._c);
                let _ct = hopeCiphertext<T> {
                    id: bson::oid::ObjectId::new(),
                    _c: _c,
                    _g: _g1,
                    _h: pairing(_g1, super::_p),
                    _o: lookup_code(_c),
                };
                match insert_apl(_ct) {
                    Some(res) => {
                        match insert_code(_ct) {
                            Some(res) => *self = _ct,
                            None => {
                                panic!("this should not happen!");
                            }
                        }
                    }
                    None => {
                        panic!("this should not happen!");
                    }
                }
            }
        }
    }
}

impl StdSub for hopeCiphertext<T> {
    type Output = hopeCiphertext<T>;

    fn sub(self, other: Point) -> hopeCiphertext<T> {
        Point {
            //x: self.x - other.x,
            //y: self.y - other.y,
            id: bson::oid::ObjectId::new(),
            _c: _ct,
            _g: _g,
            _h: _h,
            _o: _o,
        }
    }
}

impl SubAssign for hopeCiphertext<T> {
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            //x: self.x + other.x,
            //y: self.y + other.y,
            id: bson::oid::ObjectId::new(),
            _c: _ct,
            _g: _g,
            _h: _h,
            _o: _o,
        };
    }
}

impl std::ops::Add for hopeCiphertext<T> {
    type Output = hopeCiphertext<T>;

    fn add(self, other: hopeCiphertext<T>) -> hopeCiphertext<T> {
        if let Some(ek) = System::enc_key() {
            let _g1 = self._g.add(other._g);
            match super::lookup_apl(_g1) {
                Some(_ct) => {
                    return _ct;
                }
                None => {
                    let _addition = Paillier::add(&ek._key, self._c, other._c);
                    Paillier::rerandomize(&ek._key, _addition);
                    let _ct = hopeCiphertext<T> {
                        _id: ObjectId::new().unwrap(),
                        _c: _addition,
                        _g: _g1,
                        _h: pairing(_g1, super::_q),
                        _o: super::lookup_code(_c),
                    };
                    match super::insert_apl(_ct) {
                        Some(res) => {
                            match super::insert_code(_ct) {
                                Some(res) => _ct,
                                None => {
                                    panic!(
                                        "PANIC ! this should not happen! coould not insert code"
                                    );
                                }
                            }
                        }
                        None => {
                            panic!("PANIC !this should not happen! could not insert apl");
                        }
                    }
                }
            }
        } else {
            panic!("PANIC ! no encryption key found =(")
        }
    }
}
*/
impl hopeCiphertext {
    pub fn clone(
        _id: ObjectId,
        _c: EncodedCiphertext<u64>,
        _g: bn::G1,
        _h: bn::Gt,
        _o: u64,
    ) -> hopeCiphertext {
        hopeCiphertext {
            _id: _id,
            _c: _c,
            _g: _g,
            _h: _h,
            _o: _o,
        }
    }

    pub fn from_id(
        _id: ObjectId,
        _c: EncodedCiphertext<u64>,
        _g: bn::G1,
        _h: bn::Gt,
        _o: u64,
    ) -> hopeCiphertext {
        hopeCiphertext {
            _id: _id,
            _c: _c,
            _g: _g,
            _h: _h,
            _o: _o,
        }
    }

    pub fn new(_c: EncodedCiphertext<u64>, _g: bn::G1, _h: bn::Gt, _o: u64) -> hopeCiphertext {
        hopeCiphertext {
            _id: ObjectId::new().unwrap(),
            _c: _c,
            _g: _g,
            _h: _h,
            _o: _o,
        }
    }
}

impl Ord for hopeCiphertext {
    fn cmp(&self, other: &Self) -> Ordering {
        self._o.cmp(&other._o)
    }
}

impl PartialOrd for hopeCiphertext {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for hopeCiphertext {
    fn eq(&self, other: &Self) -> bool {
        self._o == other._o
    }
}

impl Eq for hopeCiphertext {}



#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn and() {

        //        assert_eq!(_match.unwrap(), _plaintext);
    }
}



use keyval::{KeyVal};
use keyval::{Attachment,SettableAttachment};
//use utils;
use transport::LocalAdd;
 use mydht_base::route::byte_rep::{
    DHTElemBytes,
  };


// reexport
pub use mydht_base::peer::*;

#[derive(RustcDecodable,RustcEncodable,Debug,PartialEq,Eq,Clone)]
/// Node using an usize as address (for use with transport tests)
pub struct PeerTest {
  pub nodeid  : String,
  pub address : LocalAdd,
}

impl KeyVal for PeerTest {
  type Key = String;
  #[inline]
  fn get_key(& self) -> Self::Key {
    self.nodeid.clone()
  }
/* 
  #[inline]
  fn get_key_ref<'a>(&'a self) -> &'a NodeID {
    &self.nodeid
  }*/
  noattachment!();
}

impl SettableAttachment for PeerTest { }

impl Peer for PeerTest {
  type Address = LocalAdd;
  fn to_address(&self) -> Self::Address {
    self.address.clone()
  }
  noshadow!();
}

impl<'a> DHTElemBytes<'a> for PeerTest {
    // return ing Vec<u8> is stupid but it is for testing
    type Bytes = Vec<u8>;
    fn bytes_ref_keb (&'a self) -> Self::Bytes {
      self.nodeid.bytes_ref_keb()
      // res.push((self.address).0 as u8); // should be key related
    }
    fn kelem_eq_keb(&self, other : &Self) -> bool {
      self.nodeid == other.nodeid
      && self.address == other.address
    }
}



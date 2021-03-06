//! test was move from base (it could not compile in base since its trait just change
//! (bidirectional dependency))
//! TODO seems pretty useless : remove??
use keyval::KeyVal;
use node::{Node,NodeID};
use peer::{Peer,Shadow};
use std::cmp::Eq;
use std::cmp::PartialEq;

use keyval::{Attachment,SettableAttachment};

use rustc_serialize::{Encodable, Encoder, Decoder};



// Testing only nodeK, with key different from id
#[derive(RustcDecodable,RustcEncodable,Debug,Clone)]
struct NodeK2(Node,String);


impl Eq for NodeK2 {}
impl PartialEq<NodeK2> for NodeK2 {

    fn eq(&self, other: &NodeK2) -> bool {
      other.0 == self.0 && other.1 == self.1
    }

}

impl KeyVal for NodeK2 {
  type Key = String;
   fn get_key(&self) -> NodeID {
        self.1.clone()
    }
/* 
    fn get_key_ref<'a>(&'a self) -> &'a NodeID {
        &self.1
    }*/
    noattachment!();
  }
  impl SettableAttachment for NodeK2 { }

  impl Peer for NodeK2 {
    type Address = <Node as Peer>::Address;
    type Shadow = <Node as Peer>::Shadow;
    #[inline]
    fn get_address(&self) -> &<Node as Peer>::Address {
      self.0.get_address()
    }
    #[inline]
    fn get_shadower (&self, write : bool) -> Self::Shadow {
      self.0.get_shadower(write)
    }
    fn default_auth_mode(&self) -> <Self::Shadow as Shadow>::ShadowMode {
     self.0.default_auth_mode() 
    }
    fn default_message_mode(&self) -> <Self::Shadow as Shadow>::ShadowMode {
       self.0.default_message_mode() 
    }
    fn default_header_mode(&self) -> <Self::Shadow as Shadow>::ShadowMode {
       self.0.default_header_mode() 
    }


  }


use rand::ThreadRng;
use rand::thread_rng;
use rand::Rng;

use std::io::{Error, ErrorKind};
use peer::{
  Peer,
  Shadow,
  ShadowSim,
};
use std::marker::PhantomData;
use mydht_base::tunnel_new::{
  TunnelCache,
  RouteProvider,
  SymProvider,
  ReplyProvider,
};
use mydht_base::tunnel_new::nope::Nope;
use mydht_base::tunnel_new::full::{
  Full,
  GenTunnelTraits,
  TunnelCachedWriterExt,
  TunnelCachedReaderExt,
  FullW,
};
use mydht_base::tunnel_new::info::multi::{
  MultipleReplyMode,
  ReplyInfoProvider,
  ReplyInfo,
};
use mydht_base::tunnel_new::info::error::{
  MultiErrorInfo,
};
use std::collections::HashMap;
use std::io::{
  Write,
  Read,
  Result,
  Cursor,
};
use rand::os::OsRng;

use readwrite_comp::{
  ExtRead,
  ExtWrite,
  CompW,
};
use peer::{
  PeerTest,
};
use transport::{
  LocalAdd,
};
use shadow::{
  ShadowTest,
  ShadowModeTest,
};
use mydht_base::bytes_wr::sized_windows::{
  SizedWindowsParams,
  SizedWindows,
};

#[derive(Clone)]
pub struct TestSizedWindows;

#[derive(Clone)]
pub struct TestSizedWindowsHead;
impl SizedWindowsParams for TestSizedWindowsHead {
    const INIT_SIZE : usize = 15;
    const GROWTH_RATIO : Option<(usize,usize)> = None;
    const WRITE_SIZE : bool = false;
    const SECURE_PAD : bool = false;
}

impl SizedWindowsParams for TestSizedWindows {
    const INIT_SIZE : usize = 45;
    const GROWTH_RATIO : Option<(usize,usize)> = Some((3,2));
    const WRITE_SIZE : bool = true;
    const SECURE_PAD : bool = false;
}

type CachedR = TunnelCachedReaderExt<SRead,SizedWindows<TestSizedWindows>>;
type CachedW = TunnelCachedWriterExt<SWrite,SizedWindows<TestSizedWindows>>;

pub struct CachedInfo {
  // TODO rename field plus rem option
  pub cached_key : Option<CachedW>,
  pub prev_peer : Vec<u8>,
}

/// simply vec as regarding algo get are done in push order most of the time
/// second usize is next get index (starting at 0), last is cache id last ix
pub struct CachedInfoManager (Vec<CachedInfo>, usize, usize);

impl CachedInfoManager {
  fn inc_ix(&mut self) {
  self.1+=1;
    if self.1 == self.0.len() {
      self.1=0;
    }
  }
}
/// TODO type for SSR 
impl TunnelCache<CachedW,CachedR> for CachedInfoManager {

  fn put_symw_tunnel(&mut self, ssw : CachedW, ppi : Vec<u8>) -> Result<()> {
    self.0.push(CachedInfo{
      cached_key : Some(ssw),
      prev_peer : ppi,
    });

  
    Ok(())
  }

  fn get_symw_tunnel(&mut self, k : &[u8]) -> Result<&mut CachedW> {


    for i in self.2 .. self.0.len() {
      if self.0[i].prev_peer == k {
        return Ok(self.0[i].cached_key.as_mut().unwrap())
      }
    };
    for i in 0 .. self.2 {
      if self.0[i].prev_peer == k {
        return Ok(self.0[i].cached_key.as_mut().unwrap())
      }
    };

    Err(Error::new(ErrorKind::Other, "Missing content : TODO change trait to return an option in result"))
  }
 
  fn has_symw_tunnel(&mut self, k : &[u8]) -> bool {
    self.get_symw_tunnel(k).is_ok()
  }

  fn put_symr_tunnel(&mut self, _ : CachedR) -> Result<Vec<u8>> {
    panic!("unimp")
  }
  fn get_symr_tunnel(&mut self, _ : &[u8]) -> Result<&mut CachedR> {
    panic!("unimp")
  }
  fn new_cache_id (&mut self) -> Vec<u8> {
  let i = self.2;
  self.2 = i + 1; 
  vec!(i as u8, (i + 1) as u8, (i + 2) as u8, (i + 3) as u8)
  }


}

#[derive(Clone)]
pub struct SProv (ShadowTest);
#[derive(Clone)]
pub struct SRead (ShadowTest);
#[derive(Clone)]
pub struct SWrite (ShadowTest);
impl ExtWrite for SWrite {
  #[inline]
  fn write_header<W : Write>(&mut self, w : &mut W) -> Result<()> {
    self.0.write_header(w)
  }
  #[inline]
  fn write_into<W : Write>(&mut self, w : &mut W, cont : &[u8]) -> Result<usize> {
    self.0.write_into(w,cont)
  }
  #[inline]
  fn flush_into<W : Write>(&mut self, w : &mut W) -> Result<()> {
    self.0.flush_into(w)
  }
  #[inline]
  fn write_end<W : Write>(&mut self, w : &mut W) -> Result<()> {
    self.0.write_end(w)
  }
}
impl ExtRead for SRead {
  fn read_header<R : Read>(&mut self, r : &mut R) -> Result<()> {
    self.0.read_header(r)
  }
  #[inline]
  fn read_from<R : Read>(&mut self, r : &mut R, buf : &mut[u8]) -> Result<usize> {
    self.0.read_from(r,buf)
  }
  #[inline]
  fn read_end<R : Read>(&mut self, r : &mut R) -> Result<()> {
    self.0.read_end(r)
  }
}

impl<P : Peer> SymProvider<SWrite,SRead,P> for SProv {
  fn new_sym_key (&mut self, p : &P) -> Vec<u8> {
    ShadowTest::shadow_simkey()
  }
  // TODO peerkey at 0??
  fn new_sym_writer (&mut self, v : Vec<u8>) -> SWrite {
    let mut st = self.0.clone();
    st.0 = v[0];
    st.1 = v[0];
    SWrite(st)
  }
  // TODO peerkey at 0??
  fn new_sym_reader (&mut self, v : Vec<u8>) -> SRead {
    let mut st = self.0.clone();
    st.0 = v[0];
    st.1 = v[0];
    SRead(st)
  }
}

#[derive(Clone)]
pub struct TunnelTestConfig<P:Peer> {
    pub me : P,
    pub dest : P,
    pub error_hop : usize, // 0 as no error, then ix of proxy hop (starting at one)
    pub nbpeer : usize,
    pub tmode : TunnelMode,
    pub input_length : usize,
    pub write_buffer_length : usize,
    pub read_buffer_length : usize,
    pub shead : <<P as Peer>::Shadow as Shadow>::ShadowMode,
    pub scont : <<P as Peer>::Shadow as Shadow>::ShadowMode,
    pub cache_ids : Vec<Vec<u8>>,
    pub reply_mode : MultipleReplyMode,
    pub error_mode : MultipleReplyMode,
}

#[derive(Clone)]
pub enum TunnelMode {
  NoTunnel,
  Tunnel,
  BiTunnel,
  BiTunnelOther,
  NoRepTunnel,
}

pub struct Rp<P : Peer>(bool,Vec<P>, Vec<P>, usize, Vec<P>);

pub struct SingleRp<P : Peer> (Vec<P>);
impl<P : Peer> Rp<P> {
  pub fn new (s : usize,pt : Vec<P>, pt2 : Vec<P>) -> Rp<P> {
    //Rp(false,peer_tests(),peer_tests_2(),s, Vec::new())
    Rp(false,pt,pt2,s, Vec::new())
  }
  pub fn set_size(&mut self, s : usize) {
    self.3 = s;
  }
}

impl<P : Peer> RouteProvider<P> for Rp<P> {
  fn new_route (&mut self, dest : &P) -> Vec<&P> {
    self.0 = !self.0;
    self.4.push(dest.clone());
    let mut r : Vec<&P> = if self.0 {
      self.1[..self.3].iter().collect()
    } else {
      self.2[..self.3].iter().collect()
    };
    r[self.3 - 1] = self.4.last().unwrap();
    r
  }
  /// for bitunnel (arg is still dest our peer address is known to route provider) 
  fn new_reply_route (&mut self, dest : &P) -> Vec<&P> {
    self.0 = !self.0;
    self.4.push(dest.clone());
    let mut r : Vec<&P> = if self.0 {
      self.1[..self.3].iter().rev().collect()
    } else {
      self.2[..self.3].iter().rev().collect()
    };
    r[0] = self.4.last().unwrap();
    r
  }
}

impl<P : Peer> RouteProvider<P> for SingleRp<P> {
  fn new_route (&mut self, dest : &P) -> Vec<&P> {
      self.0.iter().collect()
  }
  fn new_reply_route (&mut self, dest : &P) -> Vec<&P> {
      self.0.iter().rev().collect()
  }
}

/*pub trait ErrorProvider<P : Peer, EI : Info> {
  /// Error infos bases for peers
  fn new_error_route (&mut self, &[&P]) -> Vec<EI>;
}

pub struct ReplyInfoProvider<E : ExtWrite + Clone, TNR : TunnelNoRep,SSW,SSR, SP : SymProvider<SSW,SSR>, RP : RouteProvider<TNR::P>> {
  tunrep : TNR,
  // for different reply route
  symprov : SP,
  routeprov : RP,
  _p : PhantomData<(SSW,SSR)>,
}*/
struct ReplyTraits<P : Peer>(PhantomData<P>);
struct TestTunnelTraits<P : Peer>(PhantomData<P>);
impl<P : Peer> GenTunnelTraits for ReplyTraits<P> {
  type P = P;
  type SSW = Nope;
  type SSR = Nope;
  type TC = Nope;
  type LW = SizedWindows<TestSizedWindows>;
  type LR = SizedWindows<TestSizedWindows>;
  type RP = Nope;
  type RW = Nope;
  type REP = Nope;
  type EP = Nope;
}
impl<P : Peer> GenTunnelTraits for TestTunnelTraits<P> {
  type P = P;
  type SSW = SWrite;
  type SSR = SRead;
  type TC = CachedInfoManager;
  type LW = SizedWindows<TestSizedWindows>;
  type LR = SizedWindows<TestSizedWindows>;
  type RP = Rp<P>;
//pub struct ReplyInfoProvider<E : ExtWrite + Clone, TNR : TunnelNoRep,SSW,SSR, SP : SymProvider<SSW,SSR>, RP : RouteProvider<TNR::P>> {
//impl<E : ExtWrite + Clone,P : Peer,TW : TunnelWriter, TNR : TunnelNoRep<P=P,TW=TW>,SSW,SSR,SP : SymProvider<SSW,SSR>,RP : RouteProvider<P>> ReplyProvider<P, ReplyInfo<E,P,TW>,SSW,SSR> for ReplyInfoProvider<E,TNR,SSW,SSR,SP,RP> {
//
//impl<E : ExtWrite, P : Peer, RI : RepInfo, EI : Info> TunnelWriter for FullW<RI,EI,P,E> {
//type TW = FullW<ReplyInfo<TT::LW,TT::P,TT::RW>, MultiErrorInfo<TT::LW,TT::RW>, TT::P, TT::LW>;
  type RW = FullW<ReplyInfo<Self::LW,Self::P,Nope>, MultiErrorInfo<Self::LW,Nope>,Self::P, Self::LW>;
  type REP = ReplyInfoProvider<
    SizedWindows<TestSizedWindows>,
    Full<ReplyTraits<P>>,
    SWrite,
    SRead,
    SProv,
    SingleRp<P>
  >;
  type EP = Nope; // TODO
}


/// main tunnel test : send message over a route
pub fn tunnel_test<P : Peer> (mut route_prov : Rp<P>, tc : TunnelTestConfig<P>)
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{

 let mut cache : CachedInfoManager = CachedInfoManager(Vec::new(),0,0);
 let TunnelTestConfig {
     me : me,
     dest : dest,
     error_hop : error_hop,
     nbpeer : nbpeer,
     tmode : tmode,
     input_length : input_length,
     write_buffer_length : write_buffer_length,
     read_buffer_length : read_buffer_length,
     shead : shead,
     scont : scont,
     cache_ids : mut cache_ids,
     reply_mode : reply_mode,
     error_mode : error_mode,
 } = tc.clone();
 let route_rep : Vec<P> = match reply_mode {
   MultipleReplyMode::OtherRoute => route_prov.new_reply_route(&dest).into_iter().cloned().collect(),
   _ => Vec::new(),
 };
 // TODO error in reply ??
 let tunnel_reply : Full<ReplyTraits<P>> = Full {
  me : dest.clone(),
  reply_mode : MultipleReplyMode::NoHandling,
  error_mode : MultipleReplyMode::NoHandling,
  cache : Nope,
//  pub sym_prov : TT::SP,
  route_prov : Nope,
  reply_prov : Nope,
  error_prov : Nope,
  rng : thread_rng(),
  limiter_proto_w : SizedWindows::new(TestSizedWindows),
  limiter_proto_r : SizedWindows::new(TestSizedWindows),
  _p : PhantomData,
 };


 let rip = ReplyInfoProvider {
   mode : reply_mode.clone(),
   lim : SizedWindows::new(TestSizedWindows),
   tunrep : tunnel_reply,
   symprov : SProv(ShadowTest (0,0, ShadowModeTest::SimpleShift)),
   routeprov : SingleRp(route_rep),
   _p : PhantomData,
 };

 let tunnel : Full<TestTunnelTraits<P>> = Full {
  me : me,
  reply_mode : reply_mode,
  error_mode : error_mode,
  cache : cache,
//  pub sym_prov : TT::SP,
  route_prov : route_prov,
  reply_prov : rip,
  error_prov : Nope, // TODO error p
  rng : thread_rng(),
  limiter_proto_w : SizedWindows::new(TestSizedWindows),
  limiter_proto_r : SizedWindows::new(TestSizedWindows),
  _p : PhantomData,
 };


  let mut inputb = vec![0;input_length];
  let mut rnd = OsRng::new().unwrap();
  rnd.fill_bytes(&mut inputb);
  let mut output : Cursor<Vec<u8>> = Cursor::new(Vec::new());
  let input = inputb;
/*  let vec_route : Vec<(usize,&P)> = route.iter().map(|p|{
    let errorid = rnd.gen();
    (errorid,p)
  }).collect();*/
  // send message test
/*  let ocr = {
    let mut tunn_we = TunnelWriterExt::new(
      &vec_route[..],
      SizedWindows::new(TestSizedWindows),
      tmode.clone(),
      TunnelState::QueryOnce,// query once default
      None,
      shead.clone(),
      scont.clone(),
      None,// no error routes
      None,// no specific reply route
      cache_ids[0].clone(),
      Some(SizedWindows::new(TestSizedWindows)),// for cached reader
    ).unwrap();

    let mut tunn_w = tunn_we.as_writer(&mut output);
    
    //tunn_w.write_all(&input[..input_length]).unwrap();
    let mut ix = 0;
    while ix < input_length {
      if ix + write_buffer_length < input_length {
        ix += tunn_w.write(&input[ix..ix + write_buffer_length]).unwrap();
      } else {
        ix += tunn_w.write(&input[ix..]).unwrap();
      }
    }
    tunn_w.flush().unwrap();
    // TODO create tunnel reader from possible cached tunnel
    Some(())
  };*/

}

fn peer_tests () -> Vec<PeerTest> {
[ PeerTest {
    nodeid: "toid1".to_string(),
    address : LocalAdd(1),
    keyshift: 2,
},
 PeerTest  {
    nodeid: "toid2".to_string(),
    address : LocalAdd(2),
    keyshift: 3,
},
 PeerTest {
    nodeid: "toid3".to_string(),
    address : LocalAdd(3),
    keyshift: 4,
},
 PeerTest {
    nodeid: "toid4".to_string(),
    address : LocalAdd(4),
    keyshift: 5,
},
 PeerTest {
    nodeid: "toid5".to_string(),
    address : LocalAdd(5),
    keyshift: 6,
},
 PeerTest {
    nodeid: "toid6".to_string(),
    address : LocalAdd(6),
    keyshift: 5,
},
].to_vec()
}

fn peer_tests_2 () -> Vec<PeerTest> {
[ PeerTest {
    nodeid: "toid1".to_string(),
    address : LocalAdd(1),
    keyshift: 2,
},
PeerTest {
    nodeid: "toid7".to_string(),
    address : LocalAdd(7),
    keyshift: 9,
},
 PeerTest  {
    nodeid: "toid8".to_string(),
    address : LocalAdd(8),
    keyshift: 11,
},
 PeerTest {
    nodeid: "toid9".to_string(),
    address : LocalAdd(9),
    keyshift: 6,
},
 PeerTest {
    nodeid: "toid10".to_string(),
    address : LocalAdd(10),
    keyshift: 2,
},
 PeerTest {
    nodeid: "toid11".to_string(),
    address : LocalAdd(11),
    keyshift: 1,
},
].to_vec()
}



use peer::{
  Peer,
  Shadow,
  ShadowSim,
};

use std::collections::HashMap;
use std::io::{
  Write,
  Read,
  Result as IoResult,
  Cursor,
};
use rand::os::OsRng;
use rand::Rng;

use mydht_base::tunnel::{
  self,
  TunnelWriter,
  TunnelWriterExt,
  TunnelCachedWriterExt,
  TunnelCachedReaderExt,
  TunnelReader,
  TunnelReaderExt,
  TunnelMode,
  TunnelState,
  TunnelShadowMode,
  proxy_content,
  report_error,
  flush_read_on_proxy_error,
  ErrorHandlingMode,
  ErrorHandlingInfo,
};

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

/// for testing a limited cache is used
pub struct CachedInfo<P : Peer> {

  pub cached_key : Option<TunnelCachedWriterExt<SizedWindows<TestSizedWindows>,P>>,
  pub prev_peer : Vec<u8>,

}

/// main tunnel test : send message over a route
pub fn tunnel_test<P : Peer> (route : Vec<P>, tc : TunnelTestConfig<<<P as Peer>::Shadow as Shadow>::ShadowMode>)
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{

 let mut cache : Vec<CachedInfo<P>>= Vec::new();
 let TunnelTestConfig {
     error_hop : error_hop,
     nbpeer : nbpeer,
     tmode : tmode,
     input_length : input_length,
     write_buffer_length : write_buffer_length,
     read_buffer_length : read_buffer_length,
     shead : shead,
     scont : scont,
     cache_ids : mut cache_ids,
} = tc.clone();


  let route_len = route.len();

  let mut inputb = vec![0;input_length];
  let mut rnd = OsRng::new().unwrap();
  rnd.fill_bytes(&mut inputb);
  let mut output = Cursor::new(Vec::new());
  let input = inputb;
  let vec_route : Vec<(usize,&P)> = route.iter().map(|p|{
    let errorid = rnd.gen();
    (errorid,p)
  }).collect();
  // send message test
  let ocr = {
    let (mut tunn_we, mut ocr) = TunnelWriterExt::new(
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
    ocr
  };

  // middle proxy message
  /*pub fn proxy_content<
  P : Peer,
  ER : ExtRead,
  R : Read,
  EW : ExtWrite,
  W : Write> ( 
  buf : &mut [u8], 
  tre : &mut TunnelReaderExt<ER,P>,
  mut er : ER,
  mut ew : EW,
  r : &mut R,
  w : &mut W) -> Result<()> {
    {*/

  for i in 1 .. route_len - 1 {
    let mut readbuf = vec![0;read_buffer_length];

{
    let mut input_v = Cursor::new(output.into_inner());

    let mut tunn_r = TunnelReaderExt::new(route.get(i).unwrap(),SizedWindows::new(TestSizedWindows),None,None);
    //let mut tunn_r : TunnelReaderExt2<P,_> = TunnelReaderExt2::new(&mut input_v,&mut buf2[..],route.get(i).unwrap(),shead.clone(),scont.clone());
 
    assert!(tunn_r.is_dest() == None);
    tunn_r.read_header(&mut input_v).unwrap();
    assert!(tunn_r.is_dest() == Some(false));
    // check tcid (head read)
    if tunn_r.mode.do_cache() {
     assert_eq!(tunn_r.previous_cacheid, cache_ids[i-1]);
    }

    // error mgmt
    if i == error_hop {
      return tunnel_test_err (vec_route, tc, &mut tunn_r, &mut readbuf, &mut input_v, i)
    }

    output = Cursor::new(Vec::new());
    if let TunnelMode::Tunnel(..) = tunn_r.mode {
 
    if i == 1 {
      let mut buf :Vec<u8> = Vec::new();
      let mut cbuf = Cursor::new(buf);
//      try!(self.2.as_mut().unwrap().send_shadow_simkey(w)); 
      tunn_r.rep_key.as_mut().unwrap().send_shadow_simkey(&mut cbuf).unwrap();
      let mut t = cbuf.into_inner();
 
    }
    }

    // proxy message test
    proxy_content(
    &mut readbuf[..], 
    &mut tunn_r, 
    SizedWindows::new(TestSizedWindows), 
    SizedWindows::new(TestSizedWindows), 
    SizedWindows::new(TestSizedWindows), 
    SizedWindows::new(TestSizedWindows), 
    &mut input_v, 
    &mut output,
    &cache_ids[i][..],
    ).unwrap();

    // get cached key for Tunnel
//    if let TunnelMode::Tunnel(..) = tunn_r.mode {
//
    let pcid = tunn_r.previous_cacheid.clone();
    let writerex = tunn_r.rep_key.map(|rk| TunnelCachedWriterExt::new(rk, pcid, SizedWindows::new(TestSizedWindows)));
      cache.push( 
        CachedInfo {
          cached_key : writerex,
          prev_peer : tunn_r.previous_cacheid,
      });
 //   }


 }
   output.flush().unwrap();

  }

  // read message test for dest
  {
    let mut ix = 0;

    
    let mut readbuf = vec![0;read_buffer_length];
    let mut input_v = Cursor::new(output.into_inner());
    let mut tunn_re = TunnelReaderExt::new(route.get(route_len - 1).unwrap(),SizedWindows::new(TestSizedWindows),None,None);
    {
    let mut tunn_r = tunn_re.as_reader(&mut input_v);
    assert_eq!(tunn_r.1.is_dest(), None);
    let mut emptybuf = [];
    assert_eq!(0,tunn_r.read( &mut emptybuf[..]).unwrap());
    assert_eq!(tunn_r.1.is_dest(), Some(true));

/*    let mut l = 1;
     while l != 0 {
      l = tunn_r.read( &mut readbuf).unwrap();

      if ix < input.len() {
      if ix + l < input.len() { 
        assert!(&readbuf[..l] == &input[ix..ix + l]);
      } else {
        assert!(&readbuf[..input.len() - ix] == &input[ix..]);
      }
      }
      ix += l;
    }*/
    while ix < input_length { // known length
      let l = if ix + readbuf.len() < input.len() { 
        tunn_r.read( &mut readbuf).unwrap()
      } else {
        tunn_r.read( &mut readbuf[..input.len() - ix]).unwrap()
      };

      assert!(l!=0);

      assert_eq!(&readbuf[..l], &input[ix..ix + l]);
      ix += l;
    }
    if tunn_r.1.mode.do_cache() {
      // last - 1 (last is test cache id of dest (used for long term route(not query_once))
      assert_eq!(tunn_r.1.previous_cacheid, cache_ids[route_len - 2]);
    }


    //let l = tunn_r.read(&mut readbuf).unwrap();
    //assert!(l==0);
    assert!(tunn_r.read_end().is_ok());


    // check error report value (could add this to proxy to)
    match tunn_r.1.shadow.1.as_ref().unwrap().error_handle {
      ErrorHandlingInfo::NoHandling => {
        assert_eq!(tmode.errhandling_mode(), ErrorHandlingMode::NoHandling);
      },
      ErrorHandlingInfo::KnownDest(ref key, ref otm) => {
        assert_eq!(key, &route[0].get_key());
      },
      ErrorHandlingInfo::ErrorRoute(ref ecode) => {
        assert_eq!(tmode.errhandling_mode(), ErrorHandlingMode::ErrorRoute);
        assert_eq!(ecode, &vec_route.last().unwrap().0);
      },
      ErrorHandlingInfo::ErrorCachedRoute(ref ecode) => {
        assert_eq!(tmode.errhandling_mode(), ErrorHandlingMode::ErrorCachedRoute);
        assert_eq!(ecode, &vec_route.last().unwrap().0);
      },
    }

  }
   let pcid = tunn_re.previous_cacheid.clone();
   let writerex = tunn_re.rep_key.map(|rk| TunnelCachedWriterExt::new(rk, pcid, SizedWindows::new(TestSizedWindows)));
   cache.push(
        CachedInfo {
          cached_key : writerex,
          prev_peer : tunn_re.previous_cacheid,
    });

  }

  // reply
  match tmode {
    TunnelMode::Tunnel(_,_,_) => {
      // cached object : reply like error but with a key
      tunnel_rep_cached (vec_route, tc, cache, cache_ids, ocr.unwrap());


    },
    TunnelMode::BiTunnel(_,_,_,_) => {
      // TODO
    },
    _ => (),
  }

}
pub fn tunnel_rep_cached<P : Peer> (route : Vec<(usize,&P)>, tc : TunnelTestConfig<<<P as Peer>::Shadow as Shadow>::ShadowMode>, mut cache : Vec<CachedInfo<P>>, mut cache_ids:Vec<Vec<u8>>, mut dest_reader : TunnelCachedReaderExt<SizedWindows<TestSizedWindows>,P>)
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{
  let mut inputb = vec![0;tc.input_length];
  let mut rnd = OsRng::new().unwrap();
  rnd.fill_bytes(&mut inputb);
  let input_or = inputb;
  let mut input = input_or.clone();
  let mut i = cache.len();
  let send_cache_id = cache_ids.pop().unwrap();
  for cach in cache.iter_mut().rev() {
    let mut output = Cursor::new(Vec::new());
    // TODO Tunnel_Cached_Writer (write header with tunn_mode... plus encode with shadow sim)
    // TODO cachedkey as compextw (when stored) 
    let writerex : &mut TunnelCachedWriterExt<SizedWindows<TestSizedWindows>,P> = cach.cached_key.as_mut().unwrap();
    CompW::new(&mut output, writerex).write_all(&input[..]).unwrap();
                                             // send our id for long term cached Some(&(cache_ids.pop())));

    // use of tunnel_reader to have a real looking way of doing it but nicer approach is read state
    // then read key:
    //
    // let tun_state = try!(bin_decode(r, SizeLimit::Infinite).map_err(|e|BindErr(e)));
    //  if let TunnelState::ReplyCached = tun_state {
    //    cache_id = try!(bin_decode(r, SizeLimit::Infinite).map_err(|e|BindErr(e)));
    
    i = i - 1;
    let us_cache_id = cache_ids.pop().unwrap();
    let or = output.into_inner();
    let mut input_v = Cursor::new(or);
    let mut tunn_r = TunnelReaderExt::new(route.get(i).unwrap().1,SizedWindows::new(TestSizedWindows),None,None);
    tunn_r.read_header(&mut input_v).unwrap();
    assert_eq!(tunn_r.is_dest(), None);
    assert_eq!(tunn_r.state, TunnelState::ReplyCached);
    assert_eq!(tunn_r.previous_cacheid, us_cache_id);
    // proxy it
    let mut readbuf = vec![0;tc.read_buffer_length];
    if i == 0 {
      let mut cbuf = Cursor::new(Vec::new());
      let mut ri = 1;
      dest_reader.read_header(&mut input_v).unwrap();
      while ri > 0 {
        ri = dest_reader.read_from(&mut input_v, &mut readbuf).unwrap();
        cbuf.write_all(&readbuf[0..ri]).unwrap();
      }

    let res = cbuf.into_inner();
    let sor = input_or.len();
    assert_eq!(res[..sor], input_or[..sor]);


    } else {
      let mut nextinput = Cursor::new(Vec::new());
      let mut readerex : SizedWindows<TestSizedWindows> = SizedWindows::new(TestSizedWindows);
      readerex.read_header(&mut input_v).unwrap();
      let mut ri = 1;
      while ri > 0 {
        ri = readerex.read_from(&mut input_v, &mut readbuf).unwrap();
        nextinput.write_all(&readbuf[0..ri]).unwrap();
      }

      input = nextinput.into_inner();

    }
  }

  // dest (state and cache id read : from cache id we identify as dest and get our DestCachedReader
}


/// return error up to 
pub fn tunnel_test_err<P : Peer> (route : Vec<(usize,&P)>, tc : TunnelTestConfig<<<P as Peer>::Shadow as Shadow>::ShadowMode>, tunn_r : &mut TunnelReaderExt<SizedWindows<TestSizedWindows>,P>, buf : &mut [u8],input_v : &mut Cursor<Vec<u8>>, err_p : usize)
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{

 let TunnelTestConfig {
     error_hop : error_hop,
     nbpeer : nbpeer,
     tmode : tmode,
     input_length : input_length,
     write_buffer_length : write_buffer_length,
     read_buffer_length : read_buffer_length,
     shead : shead,
     scont : scont,
     cache_ids : cache_ids,
} = tc;
  // do not have start proxy so consume (last arg) is true
  let odest = flush_read_on_proxy_error(
    tunn_r,
    SizedWindows::new(TestSizedWindows), 
    input_v,
    true,
  ).unwrap();


  /*println!("err_p{}",err_p);
  for i in &route[..] {
    println!("erid: {}",i.0);
  }*/
  // send back error id
 // let error_id = tunn_r.
  let mut output = Cursor::new(Vec::new());
  // report
  report_error(buf, 
               tunn_r,
  SizedWindows::new(TestSizedWindows), 
  //mut err : ER, // only for reading of return route if needed
  input_v,
//  r : &mut R,
  &mut output,
//  w : &mut W, // w is for reply in reply route (error_read_dest has been called before)
  ).unwrap();

  // proxy it TODO factor code??
  for i in (1 .. err_p).rev() {
    let mut readbuf = vec![0;read_buffer_length];

{
    let mut input_v = Cursor::new(output.into_inner());


    output = Cursor::new(Vec::new());
    match tunnel::read_state(&mut input_v).unwrap() {
      TunnelState::QErrorCached => {
        let cid = tunnel::read_cacheid(&mut input_v).unwrap();
        assert!(cid == *cache_ids.get(i).unwrap());
        let cid_from_cache = &cache_ids.get(i - 1).unwrap();
        let cached_errcode = route.get(i).unwrap();

        tunnel::proxy_cached_err (&mut input_v, &mut output, &cid_from_cache[..], cached_errcode.0).unwrap();
      },
      TunnelState::QError => {
        panic!("TODO");
      },
      _ => panic!("Received non error state"),
    };


   }
   output.flush().unwrap();

  }

  let mut input_v = Cursor::new(output.into_inner());
  // emitter got its content
  match tunnel::read_state(&mut input_v).unwrap() {
    TunnelState::QErrorCached => {
      let cid = tunnel::read_cacheid(&mut input_v).unwrap();
      assert!(cid == *cache_ids.get(0).unwrap());
      // reading cache we know we are dest and we retrieve route
      assert_eq!(tunnel::identify_cached_errcode(&mut input_v, route).unwrap(), err_p);
    },
    TunnelState::QError => {
      panic!("TODO");
    },
    _ => panic!("Received non error state"),
  };




/*  match tunn_r.shadow.1 {
    Some(
  ErrorHandlingMode::NoHandling => (),
  ErrorHandlingMode:: => (),

  KnownDest(Option<Box<TunnelMode>>),
  ErrorRoute,
  ErrorCachedRoute,
  _ => (),
  }
*/
}


#[test]
fn tunnel_nohop_notunnel_1() {
  let input_length = 500;
  let write_buffer_length = 360;
  let read_buffer_length = 130;
  let mut route = Vec::new();
  let pt = peer_tests();
  route.push(pt[0].clone());
  route.push(pt[1].clone());
  tunnel_test(route, TunnelTestConfig::new_notunnel(input_length, write_buffer_length, read_buffer_length, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow));
}


#[test]
fn tunnel_nohop_notunnel_2() {
  let input_length = 500;
  let write_buffer_length = 130;
  let read_buffer_length = 360;
  let mut route = Vec::new();
  let pt = peer_tests();
  route.push(pt[0].clone());
  route.push(pt[1].clone());
  tunnel_test(route, TunnelTestConfig::new_notunnel(input_length, write_buffer_length, read_buffer_length, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow));

}

fn tunnel_testpeer_test(tc : TunnelTestConfig<ShadowModeTest>) {
 
  let mut route = Vec::new();
  let pt = peer_tests();
  for i in 0..tc.nbpeer {
    route.push(pt[i].clone());
  }
  tunnel_test(route, tc); 
}
#[test]
fn tunnel_nohop_noreptunnel_1() {
  tunnel_testpeer_test(TunnelTestConfig::new_norep(2, TunnelShadowMode::Last, 500, 360, 130, ShadowModeTest::SimpleShiftNoHead, ShadowModeTest::SimpleShift, ErrorHandlingMode::NoHandling));
}
#[test]
fn tunnel_nohop_noreptunnel_2() {
  tunnel_testpeer_test(TunnelTestConfig::new_norep(2, TunnelShadowMode::Full, 500, 360, 130, ShadowModeTest::SimpleShiftNoHead, ShadowModeTest::SimpleShift,ErrorHandlingMode::NoHandling));
}



#[test]
fn tunnel_nohop_noreptunnel_3() {
  tunnel_testpeer_test(TunnelTestConfig::new_norep(2, TunnelShadowMode::Last, 500, 130, 360, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow,ErrorHandlingMode::NoHandling));
}

#[test]
fn tunnel_onehop_noreptunnel_1() {
  tunnel_testpeer_test(TunnelTestConfig::new_norep(3, TunnelShadowMode::Last, 500, 360, 130, ShadowModeTest::SimpleShiftNoHead, ShadowModeTest::SimpleShift,ErrorHandlingMode::NoHandling));
}


#[test]
fn tunnel_onehop_noreptunnel_2() {
  tunnel_testpeer_test(TunnelTestConfig::new_norep(3, TunnelShadowMode::Full, 500, 130, 360, ShadowModeTest::SimpleShift, ShadowModeTest::SimpleShift,ErrorHandlingMode::NoHandling));
}

#[test]
fn tunnel_onehop_noreptunnel_3() { // TODO disable (useless)
  tunnel_testpeer_test(TunnelTestConfig::new_norep(3, TunnelShadowMode::Last, 500, 130, 360, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow,ErrorHandlingMode::NoHandling));
}

#[test]
fn tunnel_fourhop_noreptunnel_2() {
  tunnel_testpeer_test(TunnelTestConfig::new_norep(6, TunnelShadowMode::Full, 500, 130, 360, ShadowModeTest::SimpleShift, ShadowModeTest::SimpleShift,ErrorHandlingMode::NoHandling));
}

#[test]
fn tunnel_fourhop_noreptunnel_3() {
  tunnel_testpeer_test(TunnelTestConfig::new_norep(4, TunnelShadowMode::Last, 500, 130, 360, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow,ErrorHandlingMode::KnownDest(None)));
}

fn tunnel_nohop_cachedtunnel_3() {
  tunnel_testpeer_test(TunnelTestConfig::new_cached(2, TunnelShadowMode::Last, 500, 130, 360, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow,ErrorHandlingMode::ErrorCachedRoute));
}

#[test]
fn tunnel_twohop_cachedtunnel_3() {
  tunnel_testpeer_test(TunnelTestConfig::new_cached(4, TunnelShadowMode::Last, 500, 130, 360, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow,ErrorHandlingMode::ErrorCachedRoute));
}



#[test]
fn tunnel_cached_error() {
  let mut tc = TunnelTestConfig::new_norep(6, TunnelShadowMode::Full, 500, 130, 360, ShadowModeTest::SimpleShift, ShadowModeTest::SimpleShift,
     //ErrorHandlingMode::NoHandling);
     ErrorHandlingMode::ErrorCachedRoute); // TODO add test in proxy of cacheid value
//     ErrorHandlingMode::ErrorRoute);
  // error happening at last peer -1
  tc.error_hop = 6 - 2; // last proxy hop
  tunnel_testpeer_test(tc);
}

#[derive(Clone)]
pub struct TunnelTestConfig<SM> {
    pub error_hop : usize, // 0 as no error, then ix of proxy hop (starting at one)
    pub nbpeer : usize,
    pub tmode : TunnelMode,
    pub input_length : usize,
    pub write_buffer_length : usize,
    pub read_buffer_length : usize,
    pub shead : SM,
    pub scont : SM,
    pub cache_ids : Vec<Vec<u8>>,
}

pub fn test_cache_ids(nbpeer : usize) -> Vec<Vec<u8>> {

  let mut cache_ids = Vec::new();
  for i in 0 .. nbpeer {
    let cid : Vec<u8> = vec!(i as u8, (i + 1) as u8, (i + 2) as u8, (i + 3) as u8);
    cache_ids.push(cid);
  }
  cache_ids
}


impl<SM> TunnelTestConfig<SM> {
  pub fn new(nbpeer : usize, mode : TunnelMode,  input_length : usize, write_buffer_length : usize, read_buffer_length : usize, shead : SM, scont : SM) -> TunnelTestConfig<SM> {
 TunnelTestConfig {
    error_hop : 0,
     nbpeer : nbpeer,
     tmode : mode,
     input_length : input_length,
     write_buffer_length : write_buffer_length,
     read_buffer_length : read_buffer_length,
     shead : shead,
     scont : scont,
     cache_ids : test_cache_ids(nbpeer),
}
  }
pub fn new_norep(nbpeer : usize, tsmode : TunnelShadowMode,  input_length : usize, write_buffer_length : usize, read_buffer_length : usize, shead : SM, scont : SM, em : ErrorHandlingMode) -> TunnelTestConfig<SM> {

  let tmode = TunnelMode::NoRepTunnel((nbpeer as u8) - 1,tsmode, em);
  TunnelTestConfig::new(nbpeer,tmode, input_length, write_buffer_length, read_buffer_length, shead, scont)
}
pub fn new_cached(nbpeer : usize, tsmode : TunnelShadowMode,  input_length : usize, write_buffer_length : usize, read_buffer_length : usize, shead : SM, scont : SM, em : ErrorHandlingMode) -> TunnelTestConfig<SM> {

  let tmode = TunnelMode::Tunnel((nbpeer as u8) - 1,tsmode, em);
  TunnelTestConfig::new(nbpeer,tmode, input_length, write_buffer_length, read_buffer_length, shead, scont)
}

pub fn new_notunnel(input_length : usize, write_buffer_length : usize, read_buffer_length : usize, shead : SM, scont : SM) -> TunnelTestConfig<SM> {
  TunnelTestConfig::new(2,TunnelMode::NoTunnel, input_length, write_buffer_length, read_buffer_length, shead, scont)
}

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



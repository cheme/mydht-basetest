use peer::{
  Peer,
  Shadow,
};

use std::io::{
  Write,
  Read,
  Result as IoResult,
  Cursor,
};
use rand::os::OsRng;
use rand::Rng;

use mydht_base::tunnel::{
  TunnelWriter,
  TunnelWriterExt,
  TunnelReader,
  TunnelReaderExt,
  TunnelMode,
  TunnelShadowMode,
  proxy_content,
};

use readwrite_comp::{
  ExtRead,
  ExtWrite,
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

pub fn tunnel_test<P : Peer> (route : Vec<P>, input_length : usize, write_buffer_length : usize,
read_buffer_length : usize, tmode : TunnelMode,
shead : <<P as Peer>::Shadow as Shadow>::ShadowMode,
scont : <<P as Peer>::Shadow as Shadow>::ShadowMode,
)
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{
  let route_len = route.len();

  let mut inputb = vec![0;input_length];
  OsRng::new().unwrap().fill_bytes(&mut inputb);
  let mut output = Cursor::new(Vec::new());
  let input = inputb;
  let vec_route : Vec<&P> = route.iter().map(|p|p).collect();
  // send message test
  {
    let mut tunn_we = TunnelWriterExt::new(
      &vec_route[..],
      SizedWindows::new(TestSizedWindows),
      tmode.clone(),
      None,
      shead.clone(),
      scont.clone(),
    );

    let mut tunn_w = tunn_we.as_writer(&mut output);
    
    let mut ix = 0;
    while ix < input_length {
      if ix + write_buffer_length < input_length {
        ix += tunn_w.write(&input[ix..ix + write_buffer_length]).unwrap();
      } else {
        ix += tunn_w.write(&input[ix..]).unwrap();
      }
    }
    tunn_w.flush().unwrap();

  }

println!("output : {:?}",&mut output);
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
    println!("a hop start");
    let mut readbuf = vec![0;read_buffer_length];

    println!("a dest in :  : {:?}", output);
{
    let mut input_v = Cursor::new(output.into_inner());

    let mut tunn_r = TunnelReaderExt::new(route.get(i).unwrap(),SizedWindows::new(TestSizedWindows),None);
    //let mut tunn_r : TunnelReaderExt2<P,_> = TunnelReaderExt2::new(&mut input_v,&mut buf2[..],route.get(i).unwrap(),shead.clone(),scont.clone());
 
    assert!(tunn_r.is_dest() == None);
    tunn_r.read_header(&mut input_v).unwrap();
    assert!(tunn_r.is_dest() == Some(false));

    output = Cursor::new(Vec::new());

    // proxy message test
    proxy_content(&mut readbuf[..], &mut tunn_r, 
    SizedWindows::new(TestSizedWindows), 
    SizedWindows::new(TestSizedWindows), 
    SizedWindows::new(TestSizedWindows), &mut input_v, &mut output).unwrap();
 }
   output.flush().unwrap();

  }
println!("dest!!");
  // read message test for dest
  {
    let mut ix = 0;

    
    let mut readbuf = vec![0;read_buffer_length];
    println!("a dest in :  : {:?}", output);
    let mut input_v = Cursor::new(output.into_inner());
    let mut tunn_re = TunnelReaderExt::new(route.get(route_len - 1).unwrap(),SizedWindows::new(TestSizedWindows),None);
    let mut tunn_r = tunn_re.as_reader(&mut input_v);
    assert!(tunn_r.1.is_dest() == None);
    let mut emptybuf = [];
    tunn_r.read( &mut emptybuf[..]).unwrap();
    assert!(tunn_r.1.is_dest() == Some(true));

/*    let mut l = 1;
     while l != 0 {
      println!("bfe read");
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

      assert!(&readbuf[..l] == &input[ix..ix + l]);
      ix += l;
    }

    //let l = tunn_r.read(&mut readbuf).unwrap();
    //assert!(l==0);
    assert!(tunn_r.read_end().is_ok());
  }

}

#[test]
fn tunnel_nohop_notunnel_1() {
  let tmode = TunnelMode::NoTunnel;
  let input_length = 500;
  let write_buffer_length = 360;
  let read_buffer_length = 130;
  let mut route = Vec::new();
  let pt = peer_tests();
  route.push(pt[0].clone());
  route.push(pt[1].clone());
  tunnel_test(route, input_length, write_buffer_length, read_buffer_length, tmode, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow); 
}


#[test]
fn tunnel_nohop_notunnel_2() {
  let tmode = TunnelMode::NoTunnel;
  let input_length = 500;
  let write_buffer_length = 130;
  let read_buffer_length = 360;
  let mut route = Vec::new();
  let pt = peer_tests();
  route.push(pt[0].clone());
  route.push(pt[1].clone());
  tunnel_test(route, input_length, write_buffer_length, read_buffer_length, tmode, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow); 

}

fn tunnel_public_test(nbpeer : usize, tmode : TunnelShadowMode, input_length : usize, write_buffer_length : usize, read_buffer_length : usize, shead : ShadowModeTest, scont : ShadowModeTest) {
  let tmode = TunnelMode::PublicTunnel((nbpeer as u8) - 1,tmode);
  let mut route = Vec::new();
  let pt = peer_tests();
  for i in 0..nbpeer {
    route.push(pt[i].clone());
  }
  tunnel_test(route, input_length, write_buffer_length, read_buffer_length, tmode, shead, scont); 
}
#[test]
fn tunnel_nohop_publictunnel_1() {
  tunnel_public_test(2, TunnelShadowMode::Last, 500, 360, 130, ShadowModeTest::SimpleShiftNoHead, ShadowModeTest::SimpleShift);
}
#[test]
fn tunnel_nohop_publictunnel_2() {
  tunnel_public_test(2, TunnelShadowMode::Full, 500, 360, 130, ShadowModeTest::SimpleShiftNoHead, ShadowModeTest::SimpleShift);
}



#[test]
fn tunnel_nohop_publictunnel_3() {
  tunnel_public_test(2, TunnelShadowMode::Last, 500, 130, 360, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow);
}

#[test]
fn tunnel_onehop_publictunnel_1() {
  tunnel_public_test(3, TunnelShadowMode::Last, 500, 360, 130, ShadowModeTest::SimpleShiftNoHead, ShadowModeTest::SimpleShift);
}


#[test]
fn tunnel_onehop_publictunnel_2() {
  tunnel_public_test(3, TunnelShadowMode::Full, 500, 130, 360, ShadowModeTest::SimpleShift, ShadowModeTest::SimpleShift);
}

#[test]
fn tunnel_onehop_publictunnel_3() { // TODO disable (useless)
  tunnel_public_test(3, TunnelShadowMode::Last, 500, 130, 360, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow);
}

#[test]
fn tunnel_fourhop_publictunnel_2() {
  tunnel_public_test(6, TunnelShadowMode::Full, 500, 130, 360, ShadowModeTest::SimpleShift, ShadowModeTest::SimpleShift);
}

#[test]
fn tunnel_fourhop_publictunnel_3() {
  tunnel_public_test(4, TunnelShadowMode::Last, 500, 130, 360, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow);
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



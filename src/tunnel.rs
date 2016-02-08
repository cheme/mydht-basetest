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
  TunnelReader,
  TunnelProxy,
  TunnelMode,
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
  let mut w_buf = vec![0;write_buffer_length];
  let vec_route : Vec<&P> = route.iter().map(|p|p).collect();
  // send message test
  {
    let mut tunn_w = TunnelWriter::new(
      &vec_route[..],
      tmode.clone(),
      &mut w_buf[..], 
      &mut output,
      None,
      shead.clone(),
      scont.clone(),
    );


    let mut ix = 0;
    while ix < input_length {
      if ix + write_buffer_length < input_length {
        ix += tunn_w.write(&input[ix..ix + write_buffer_length], ).unwrap();
      } else {
        ix += tunn_w.write(&input[ix..]).unwrap();
      }
    }
    tunn_w.flush().unwrap();

  }

  // middle proxy message
  for i in 1 .. route_len - 1 {
    let mut ix = 0;

    
    println!("a hop start");
    let mut readbuf = vec![0;read_buffer_length];

    let mut buf2 = vec![0;read_buffer_length]; // TODO remove
    println!("a dest in :  : {:?}", output);
{ 
    let mut input_v = Cursor::new(output.into_inner());
    let mut tunn_r : TunnelReader<P,_> = TunnelReader::new(&mut input_v,&mut buf2[..],route.get(i).unwrap(),shead.clone(),scont.clone());
 
    assert!(tunn_r.is_dest() == None);
    let mut emptybuf = [];
    tunn_r.read( &mut emptybuf[..]).unwrap();
    assert!(tunn_r.is_dest() == Some(false));

    println!("a hop not dest");
    output = Cursor::new(Vec::new());

    // proxy message test
    {
      let mut tun_prox = TunnelProxy::new(&mut tunn_r, &mut readbuf, &mut output);
      let mut ix = 1;
      while ix > 0 {
        ix = tun_prox.tunnel_proxy().unwrap();
      }
    }
}    output.flush().unwrap();

  }

  // read message test for dest
  {
    let mut ix = 0;

    
    let mut readbuf = vec![0;read_buffer_length];
let mut buf2 = vec![0;read_buffer_length]; // TODO remove
    println!("a dest in :  : {:?}", output);
    let mut input_v = Cursor::new(output.into_inner());
    let mut tunn_r : TunnelReader<P,_> = TunnelReader::new(&mut input_v,&mut buf2[..],route.get(route_len - 1).unwrap(),shead.clone(),scont.clone());

    assert!(tunn_r.is_dest() == None);
    let mut emptybuf = [];
    tunn_r.read( &mut emptybuf[..]).unwrap();
    assert!(tunn_r.is_dest() == Some(true));

    while ix < input_length {
      let l = tunn_r.read( &mut readbuf).unwrap();
      assert!(l!=0);

      println!("{:?}",&input[ix..ix + l]);
      println!("{:?}",&readbuf[..l]);
      assert!(&readbuf[..l] == &input[ix..ix + l]);
      ix += l;
    }

    let l = tunn_r.read(&mut readbuf).unwrap();
    assert!(l==0);
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
/* retest when layered ok
#[test]
fn tunnel_nohop_publictunnel_1() {
  let tmode = TunnelMode::PublicTunnel(1, TunnelShadowMode::Last);
  let input_length = 500;
  let write_buffer_length = 360;
  let read_buffer_length = 130;
  let mut route = Vec::new();
  let pt = peer_tests();
  route.push(pt[0].clone());
  route.push(pt[1].clone());
  tunnel_test(route, input_length, write_buffer_length, read_buffer_length, tmode, ShadowModeTest::SimpleShiftNoHead, ShadowModeTest::SimpleShift); 

}
#[test]
fn tunnel_nohop_publictunnel_2() {
  let tmode = TunnelMode::PublicTunnel(1, ShadowModeTest::SimpleShift, ShadowModeTest::SimpleShift);
  let input_length = 500;
  let write_buffer_length = 130;
  let read_buffer_length = 360;
  let mut route = Vec::new();
  let pt = peer_tests();
  route.push(pt[0].clone());
  route.push(pt[1].clone());
  tunnel_test(route, input_length, write_buffer_length, read_buffer_length, tmode); 

}
#[test]
fn tunnel_onehop_publictunnel_2() {
  let tmode = TunnelMode::PublicTunnel(1, ShadowModeTest::SimpleShift, ShadowModeTest::SimpleShift);
  let input_length = 500;
  let write_buffer_length = 130;
  let read_buffer_length = 360;
  let mut route = Vec::new();
  let pt = peer_tests();
  route.push(pt[0].clone());
  route.push(pt[1].clone());
  route.push(pt[2].clone());
  tunnel_test(route, input_length, write_buffer_length, read_buffer_length, tmode); 

}

#[test]
fn tunnel_nohop_publictunnel_3() {
  let tmode = TunnelMode::PublicTunnel(1, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow);
  let input_length = 500;
  let write_buffer_length = 130;
  let read_buffer_length = 360;
  let mut route = Vec::new();
  let pt = peer_tests();
  route.push(pt[0].clone());
  route.push(pt[1].clone());
  tunnel_test(route, input_length, write_buffer_length, read_buffer_length, tmode); 

}


#[test]
fn tunnel_onehop_publictunnel_3() {
  let tmode = TunnelMode::PublicTunnel(1, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow);
  let input_length = 500;
  let write_buffer_length = 130;
  let read_buffer_length = 360;
  let mut route = Vec::new();
  let pt = peer_tests();
  route.push(pt[0].clone());
  route.push(pt[1].clone());
  route.push(pt[2].clone());
  tunnel_test(route, input_length, write_buffer_length, read_buffer_length, tmode); 

}
#[test]
fn tunnel_fourhop_publictunnel_3() {
  let tmode = TunnelMode::PublicTunnel(1, ShadowModeTest::NoShadow, ShadowModeTest::NoShadow);
  let input_length = 500;
  let write_buffer_length = 130;
  let read_buffer_length = 360;
  let mut route = Vec::new();
  let pt = peer_tests();
  route.push(pt[0].clone());
  route.push(pt[1].clone());
  route.push(pt[2].clone());
  route.push(pt[3].clone());
  route.push(pt[4].clone());
  route.push(pt[5].clone());
  tunnel_test(route, input_length, write_buffer_length, read_buffer_length, tmode); 

}
*/


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



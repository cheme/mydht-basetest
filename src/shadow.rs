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
use readwrite_comp::{
  ExtRead,
  ExtWrite,
};
use std::num::Wrapping;
use rand::thread_rng;
use rand::Rng;

/// Test shadowing, do not use (slow, clear password in header).
/// Designed for testing (simply increment u8 with peer key).
/// First u8 is peer key and second one is transaction key (if used).
pub struct ShadowTest (pub u8, pub u8, pub ShadowModeTest);

#[derive(Debug,RustcDecodable,RustcEncodable,Clone,PartialEq,Eq)]
pub enum ShadowModeTest {
  NoShadow,
  SimpleShift,
  SimpleShiftNoHead,
}

#[inline]
pub fn shift_up(init : u8, inc : u8) -> u8 {
  (Wrapping(init) + Wrapping(inc)).0
}
#[inline]
pub fn shift_down(init : u8, dec : u8) -> u8 {
  (Wrapping(init) - Wrapping(dec)).0
}

impl Shadow for ShadowTest {


  type ShadowMode = ShadowModeTest;


  fn set_mode (&mut self, sm : Self::ShadowMode) {
    self.2 = sm
  }
  fn get_mode (&self) -> Self::ShadowMode {
    self.2.clone()
  }
  #[inline]
  fn shadow_iter_sim<W : Write> (&mut self, k : &[u8], vals : &[u8], w : &mut W) -> IoResult<usize> {
    match self.2 {
      ShadowModeTest::NoShadow => w.write(vals),
      _ => {
        let v2 = &mut vals.to_vec()[..];
        for i in v2.iter_mut() {
          *i = shift_up(*i,k[0]);
        }
        w.write(v2)
      },
    }
  }
  #[inline]
  fn shadow_sim_flush<W : Write> (&mut self, w : &mut W) -> IoResult<()> {
    Ok(())
  }
  #[inline]
  fn read_shadow_iter_sim<R : Read> (&mut self, k : &[u8], r : &mut R, buf: &mut [u8]) -> IoResult<usize> {
    let nb = try!(r.read(buf));
    if nb == 0 {
      return Ok(nb);
    }
    match self.2 {
      ShadowModeTest::NoShadow => Ok(nb),
      _ => {
        let v2 = &mut buf[..nb];
        for i in v2.iter_mut() {
          *i = shift_down(*i,k[0]);
        }
        Ok(nb)
      },
    }
  }

  fn shadow_simkey(&mut self) -> Vec<u8> {
    let mut res = vec![0;1];
    thread_rng().fill_bytes(&mut res);
    res
  }
 
}
impl ExtWrite for ShadowTest {

  /// write transaction key
  fn write_header<W : Write>(&mut self, w : &mut W) -> IoResult<()> {
    match self.2 {
      ShadowModeTest::NoShadow => {
        try!(w.write(&[0]));
      },
      ShadowModeTest::SimpleShift => {
        self.1 = (self.shadow_simkey())[0];
        println!("OKKKKKKKKKHead {}", self.1);
        try!(w.write(&[1,shift_up(self.1,self.0)]));
      },
      ShadowModeTest::SimpleShiftNoHead => {
        try!(w.write(&[2]));
      },
    }
    Ok(())
  }


  fn write_into<W : Write>(&mut self, w : &mut W, cont : &[u8]) -> IoResult<usize> {
    let k = match self.2 {
      ShadowModeTest::NoShadow => Vec::new(),
      ShadowModeTest::SimpleShift => {
        println!("OKKKKKKKKK {}", self.1);
        vec!(self.1)
      },
      ShadowModeTest::SimpleShiftNoHead => vec!(self.0),
    };

    self.shadow_iter_sim(&k[..], cont, w)
  }


  #[inline]
  fn flush_into<W : Write>(&mut self, w : &mut W) -> IoResult<()> {Ok(())}
  #[inline]
  fn write_end<W : Write>(&mut self, _ : &mut W) -> IoResult<()> {Ok(())}
}
impl ExtRead for ShadowTest {
  fn read_header<R : Read>(&mut self, r : &mut R) -> IoResult<()> {
    let buf = &mut [9];
    let nb = try!(r.read(buf));
    assert!(nb == 1);
    let sm : u8 = buf[0];
    let mode = if sm == 0 {
      ShadowModeTest::NoShadow
    }else if sm == 1 {
      
    let nb = try!(r.read(buf));
    assert!(nb == 1);
    let key : u8 = shift_down(buf[0],self.0);
    self.1 = key;
      ShadowModeTest::SimpleShift
    } else if sm == 2 {
      ShadowModeTest::SimpleShiftNoHead
    } else {
      panic!("wrong test shadow mode enc"); // TODO replace by err
    };
    self.2 = mode;
    Ok(())
  }
  #[inline]
  /// read shadow returning number of bytes read, probably using an internal buffer
  fn read_from<R : Read>(&mut self, r : &mut R, buf : &mut[u8]) -> IoResult<usize> {
    let k = match self.2 {
      ShadowModeTest::NoShadow => Vec::new(),
      ShadowModeTest::SimpleShift => vec!(self.1),
      ShadowModeTest::SimpleShiftNoHead => vec!(self.0),
    };
    self.read_shadow_iter_sim(&k[..], r, buf)
  }
  #[inline]
  fn read_end<R : Read>(&mut self, _ : &mut R) -> IoResult<()> {Ok(())}
}



pub fn shadower_test<P : Peer> (to_p : P, input_length : usize, write_buffer_length : usize,
read_buffer_length : usize, smode : <<P as Peer>::Shadow as Shadow>::ShadowMode) 
where <<P as Peer>::Shadow as Shadow>::ShadowMode : Eq
{

  let mut inputb = vec![0;input_length];
  thread_rng().fill_bytes(&mut inputb);
  let mut output = Cursor::new(Vec::new());
  let input = inputb;
  let mut from_shad = to_p.get_shadower(true);
  from_shad.set_mode(smode.clone());
  let mut to_shad = to_p.get_shadower(false);
  to_shad.set_mode(smode.clone());

  // sim test
  let mut ix = 0;
  let k = from_shad.shadow_simkey();
  while ix < input_length {
    if ix + write_buffer_length < input_length {
      ix += from_shad.shadow_iter_sim(&k[..],&input[ix..ix + write_buffer_length], &mut output).unwrap();
    } else {
      ix += from_shad.shadow_iter_sim(&k[..],&input[ix..], &mut output).unwrap();
    }
  }
  from_shad.shadow_sim_flush(&mut output).unwrap();
  ix = 0;
  let mut readbuf = vec![0;read_buffer_length];

  let mut input_v = Cursor::new(output.into_inner());
  while ix < input_length {
    let l = to_shad.read_shadow_iter_sim(&k[..],&mut input_v, &mut readbuf).unwrap();
    assert!(l!=0);

    println!("{:?}",&input[ix..ix + l]);
    println!("{:?}",&readbuf[..l]);
    assert!(&readbuf[..l] == &input[ix..ix + l]);
    ix += l;
  }

  let l = to_shad.read_shadow_iter_sim(&k[..],&mut input_v, &mut readbuf).unwrap();
  assert!(l==0);




  // message test
  output = Cursor::new(Vec::new());
  from_shad.shadow_header(&mut output).unwrap();
  ix = 0;
  while ix < input_length {
    if ix + write_buffer_length < input_length {
      ix += from_shad.shadow_iter(&input[ix..ix + write_buffer_length], &mut output).unwrap();
    } else {
      ix += from_shad.shadow_iter(&input[ix..], &mut output).unwrap();
    }
  }
  from_shad.shadow_flush(&mut output).unwrap();
  output.flush();

  input_v = Cursor::new(output.into_inner());

  to_shad.read_shadow_header(&mut input_v).unwrap();
  let mode = to_shad.get_mode();
  assert!(smode == mode);


  ix = 0;
  let mut readbuf = vec![0;read_buffer_length];
  while ix < input_length {
    let l = to_shad.read_shadow_iter(&mut input_v, &mut readbuf).unwrap();
    assert!(l!=0);

    println!("{:?}",&input[ix..ix + l]);
    println!("{:?}",&readbuf[..l]);
    assert!(&readbuf[..l] == &input[ix..ix + l]);
    ix += l;
  }

  let l = to_shad.read_shadow_iter(&mut input_v, &mut readbuf).unwrap();
  assert!(l==0);

}
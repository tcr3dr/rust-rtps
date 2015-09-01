#![allow(dead_code)]
#![allow(unused_must_use)]
#![allow(unused_imports)]

extern crate rustc_serialize;
extern crate mio;

#[macro_use]
extern crate log;

mod rtps;

struct Entity;

struct QosPolicy;

struct Listener;

struct StatusKind;

trait EntityTrait {
  fn set_qos(qos_list:&[QosPolicy]) -> ReturnCode_t;
  fn get_qos(qos_list:&mut [QosPolicy]) -> ReturnCode_t;
  fn set_listener(a_listener:&Listener, mask:&[StatusKind]) -> ReturnCode_t;
}

#[allow(non_camel_case_types)]
enum ReturnCode_t {
  OK,
  ERROR,
  BAD_PARAMETER,
  UNSUPPORTED,
  ALREADY_DELETED,
  OUT_OF_RESOURCES,
  NOT_ENABLED,
  IMMUTABLE_POLICY,
  INCONSISTENT_POLICY,
  PRECONDITION_NOT_MET,
  TIMEOUT,
  ILLEGAL_OPERATION,
  NO_DATA,
}

#[test]
fn it_works() {
}

#[no_mangle]
pub extern fn rmw_get_implementation_identifier() -> *const u8 {
    "libdds\0".as_ptr()
}


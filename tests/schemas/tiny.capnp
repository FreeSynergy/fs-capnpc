@0xdbb9ad1f14bf0b36;

struct Person {
  name @0 :Text;
  age @1 :UInt8;
  flag @2 :Bool;
  addr :union {
    a @3 :Int32;
    b @4 :Text;
  }
}

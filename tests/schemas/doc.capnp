@0xdbb9ad1f14bf0b38;
# file doc
struct Foo {
  # struct doc
  a @0 :Int32;  # field a doc
  b @1 :Text;   # field b doc
  u :union {
    x @2 :Int8;  # x doc
    y @3 :Int8;
  }  # union doc
}
enum E {
  one @0;  # one doc
  two @1;
}

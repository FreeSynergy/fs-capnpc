@0xdbb9ad1f14bf0c01;
struct All { int16Field @0 :Int16; }
struct G(Foo, Bar) {
  foo @0 :Foo;
  bar @1 :Bar;
  using AliasFoo = Foo;
  struct Inner { foo @0 :Foo; bar @1 :Bar; }
  using AliasInner = Inner;
}
struct U {
  a @0 :G(All, Text);
  b @1 :G(All, Text).AliasFoo = (int16Field = 123);
}

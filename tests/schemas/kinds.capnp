@0xdbb9ad1f14bf0b39;
struct S { a @0 :Int32; }
enum E { a @0; b @1; }
interface I {
  foo @0 (x :Int32) -> (y :Int32);
}
const c :Int32 = 5;
annotation ann(field, struct) :Text;
using Foo = S;

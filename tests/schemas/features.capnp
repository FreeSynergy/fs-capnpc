@0xdbb9ad1f14bf0e01;
using Cxx = import "/capnp/c++.capnp";
using Rust = import "/capnp/rust.capnp";
$Cxx.namespace("features::test");

# A file level doc comment.

enum Color {
  # The colours.
  red @0;   # trailing red
  green @1;
  blue @2 $Cxx.name("bluish");
}

annotation tag(*) :Text;
annotation flag(field, struct, enum, enumerant, union, group, interface, method, param, const, annotation, file) :Void;

const answer :UInt32 = 42;
const negative :Int64 = -9000000000;
const pi :Float64 = 3.14159;
const smallFloat :Float32 = 1e-3;
const greeting :Text = "hello \"world\"\n\t\x41é";
const blob :Data = 0x"deadbeef";
const colors :List(Color) = [red, green, blue];
const matrix :List(List(Int32)) = [[1, 2], [3], []];
const flags :List(Bool) = [true, false, true, true, false, false, true, false, true];
const nested :Point = (x = 1, y = -2, label = "p", tags = ["a", "b"]);
const points :List(Point) = [(x = 1, y = 2), (x = 3, y = 4, label = "q")];
const infinity :Float64 = inf;
const everything :Everything = (u8 = 255, i8 = -128, b = true, c = blue, text = "t", data = 0x"01 02");

struct Point $tag("pt") {
  x @0 :Int32;
  y @1 :Int32 = 7;
  label @2 :Text = "origin";
  tags @3 :List(Text);
}

struct Everything {
  void @0 :Void;
  b @1 :Bool;
  i8 @2 :Int8 = -5;
  i16 @3 :Int16;
  i32 @4 :Int32;
  i64 @5 :Int64;
  u8 @6 :UInt8;
  u16 @7 :UInt16;
  u32 @8 :UInt32;
  u64 @9 :UInt64 = 18446744073709551615;
  f32 @10 :Float32 = 1.5;
  f64 @11 :Float64 = -2.5e10;
  text @12 :Text;
  data @13 :Data;
  c @14 :Color = green;
  li @15 :List(Int32) = [1, 2, 3];
  ls @16 :List(Text);
  lp @17 :List(Point);
  any @18 :AnyPointer;
  anyStruct @19 :AnyStruct;
  anyList @20 :AnyList;
  cap @21 :Capability;
  more @22 :Bool = true;
  more2 @23 :Bool;
  more3 @24 :Bool;
  tail @25 :UInt8;
}

struct Unions {
  before @0 :UInt16;
  union {
    a @1 :UInt32;
    b @2 :Text;
    c @3 :Void;
    g :group {
      x @4 :UInt8;
      y @5 :Text;
    }
  }
  middle @6 :Bool;
  other :union {
    p @7 :UInt64;
    q @8 :List(UInt8);
  }
  grp :group $flag {
    deep @9 :Int32;
    inner :group {
      deeper @10 :Point;
    }
  }
}

struct Generic(T, U) {
  t @0 :T;
  u @1 :U;
  pair @2 :Generic(U, T);
  struct Nested { t @0 :T; }
  nested @3 :Nested;
}

interface Base $flag {
  ping @0 () -> ();
}

interface Service(Req, Resp) extends(Base) {
  call @0 (req :Req, count :UInt32 = 5) -> (resp :Resp, ok :Bool) $flag;
  stream @1 (chunk :Data) -> stream;
  generic @2 [X] (x :X) -> (y :X);
  named @3 Point -> Point;
  empty @4 ();
  getSelf @5 () -> (me :Service(Req, Resp));
}

interface Multi extends(Base, Service(Text, Data)) {
  extra @0 (p :Point) -> (e :Everything);
}

struct Holder {
  svc @0 :Service(Text, Point);
  gen @1 :Generic(Everything, Text);
  inner @2 :Generic(Point, Text).Nested;
}

using PointAlias = Point;
using import "/capnp/schema.capnp".Node;

struct UsesAlias {
  p @0 :PointAlias;
  n @1 :Node;
}

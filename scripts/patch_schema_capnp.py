#!/usr/bin/env python3
"""Post-process the generated schema_capnp.rs: add `value::Builder::set_null_variant`, which selects a
pointer-typed union variant of `Schema.Value` while leaving its pointer null (the official compiler
produces such values for default-valued Text/Data/List/Struct)."""
import sys

path = sys.argv[1]
src = open(path).read()
marker = "pub mod value {"
i = src.index(marker)
j = src.index("impl <'a,> Builder<'a,>  {", i)
j_end = j + len("impl <'a,> Builder<'a,>  {")
helper = """
        /// Select union variant `discriminant` (a pointer type) and leave its pointer null.
        #[inline]
        pub fn set_null_variant(&mut self, discriminant: u16) {
            self.builder.set_data_field::<u16>(0, discriminant);
            let mut p = self.builder.reborrow().get_pointer_field(0);
            p.clear();
        }
"""
if "set_null_variant" not in src:
    src = src[:j_end] + helper + src[j_end:]
open(path, "w").write(src)

//! Conversion of a branded declaration into a schema `Type` (`compileAsType`).
//!
//! Pattern: Strategy — behaviour depends on the kind of declaration being compiled.

use crate::compiler::*;
use crate::generics::BrandedDecl;
use crate::schema::{AnyPointer, Type, Unconstrained};

impl BrandedDecl {
    /// `compileAsType`: returns the schema type, or `None` after reporting an error.
    pub fn compile_as_type(&self, c: &mut Compiler, module: usize) -> Option<Type> {
        match self.kind() {
            Some(kind) => match kind {
                DeclKind::Enum => {
                    let (type_id, brand) = self.id_and_brand(c);
                    Some(Type::Enum { type_id, brand })
                }
                DeclKind::Struct => {
                    let (type_id, brand) = self.id_and_brand(c);
                    Some(Type::Struct { type_id, brand })
                }
                DeclKind::Interface => {
                    let (type_id, brand) = self.id_and_brand(c);
                    Some(Type::Interface { type_id, brand })
                }
                DeclKind::BuiltinList => {
                    let Some(param) = self.get_list_param() else {
                        self.add_error(c, module, "'List' requires exactly one parameter.");
                        return None;
                    };
                    let elem = param.compile_as_type(c, module)?;
                    if let Type::AnyPointer(AnyPointer::Unconstrained(u)) = &elem {
                        match u {
                            Unconstrained::AnyKind => {
                                self.add_error(c, module, "'List(AnyPointer)' is not supported.");
                                return None;
                            }
                            Unconstrained::Struct => {
                                self.add_error(c, module, "'List(AnyStruct)' is not supported.");
                                return None;
                            }
                            _ => {}
                        }
                    }
                    Some(Type::List(Box::new(elem)))
                }
                DeclKind::BuiltinVoid => Some(Type::Void),
                DeclKind::BuiltinBool => Some(Type::Bool),
                DeclKind::BuiltinInt8 => Some(Type::Int8),
                DeclKind::BuiltinInt16 => Some(Type::Int16),
                DeclKind::BuiltinInt32 => Some(Type::Int32),
                DeclKind::BuiltinInt64 => Some(Type::Int64),
                DeclKind::BuiltinUInt8 => Some(Type::UInt8),
                DeclKind::BuiltinUInt16 => Some(Type::UInt16),
                DeclKind::BuiltinUInt32 => Some(Type::UInt32),
                DeclKind::BuiltinUInt64 => Some(Type::UInt64),
                DeclKind::BuiltinFloat32 => Some(Type::Float32),
                DeclKind::BuiltinFloat64 => Some(Type::Float64),
                DeclKind::BuiltinText => Some(Type::Text),
                DeclKind::BuiltinData => Some(Type::Data),
                DeclKind::BuiltinObject => {
                    self.add_error(c, module, "As of Cap'n Proto 0.4, 'Object' has been renamed to 'AnyPointer'.  Sorry for the inconvenience, and thanks for being an early adopter.  :)");
                    Some(Type::AnyPointer(AnyPointer::Unconstrained(
                        Unconstrained::AnyKind,
                    )))
                }
                DeclKind::BuiltinAnyPointer => Some(Type::AnyPointer(AnyPointer::Unconstrained(
                    Unconstrained::AnyKind,
                ))),
                DeclKind::BuiltinAnyStruct => Some(Type::AnyPointer(AnyPointer::Unconstrained(
                    Unconstrained::Struct,
                ))),
                DeclKind::BuiltinAnyList => Some(Type::AnyPointer(AnyPointer::Unconstrained(
                    Unconstrained::List,
                ))),
                DeclKind::BuiltinCapability => Some(Type::AnyPointer(AnyPointer::Unconstrained(
                    Unconstrained::Capability,
                ))),
                _ => {
                    let msg = format!("'{}' is not a type.", self.to_string());
                    self.add_error(c, module, msg);
                    None
                }
            },
            None => {
                let ResolveResult::Param(var) = &self.body else {
                    unreachable!()
                };
                if var.id == 0 {
                    Some(Type::AnyPointer(AnyPointer::ImplicitMethodParameter {
                        parameter_index: var.index as u16,
                    }))
                } else {
                    Some(Type::AnyPointer(AnyPointer::Parameter {
                        scope_id: var.id,
                        parameter_index: var.index as u16,
                    }))
                }
            }
        }
    }
}

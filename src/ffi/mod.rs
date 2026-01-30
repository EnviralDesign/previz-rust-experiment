//! Raw FFI bindings to Filament
//! 
//! These are generated during build time and define the C API interface.
//! All functions are unsafe and operate on raw pointers.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::all)]

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

//
// Copyright 2026 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//! Derive macros for PECS (Persistent Entity Component System).
//!
//! This crate provides procedural macros to reduce boilerplate when working with PECS.
//!
//! # Examples
//!
//! ```ignore
//! use pecs::Component;
//!
//! #[derive(Component)]
//! struct Position {
//!     x: f32,
//!     y: f32,
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derives the `Component` trait for a type.
///
/// This macro automatically implements the `Component` trait, which requires
/// the type to be `'static + Send + Sync`.
///
/// # Examples
///
/// ```ignore
/// use pecs::Component;
///
/// #[derive(Component)]
/// struct Position {
///     x: f32,
///     y: f32,
/// }
///
/// #[derive(Component)]
/// struct Velocity {
///     x: f32,
///     y: f32,
/// }
/// ```
///
/// # Requirements
///
/// The type must satisfy the following bounds:
/// - `'static`: No non-'static references
/// - `Send`: Can be sent between threads
/// - `Sync`: Can be shared between threads
///
/// These bounds are automatically checked by the compiler when the macro is applied.
#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    // Build where clause with Component bounds
    let generics = &input.generics;
    let (_impl_generics, ty_generics, _where_clause) = generics.split_for_impl();
    
    // Add Send + Sync + 'static bounds for generic parameters
    let mut generics_with_bounds = generics.clone();
    for param in &mut generics_with_bounds.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param.bounds.push(syn::parse_quote!(::std::marker::Send));
            type_param.bounds.push(syn::parse_quote!(::std::marker::Sync));
            type_param.bounds.push(syn::parse_quote!('static));
        }
    }
    let (impl_generics_with_bounds, _, where_clause_with_bounds) = generics_with_bounds.split_for_impl();

    // Generate the Component trait implementation
    let expanded = quote! {
        impl #impl_generics_with_bounds ::pecs::Component for #name #ty_generics #where_clause_with_bounds {}
    };

    TokenStream::from(expanded)
}

// Made with Bob

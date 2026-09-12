use definition_rs::{CANMsg, NetField, OdysseyMsg};
use heck::{AsPascalCase, AsSnakeCase};
use proc_macro::TokenStream;
use quote::quote;
use std::fs;
use syn::Ident;

extern crate proc_macro;

const CANGEN_SPEC_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/Odyssey-Definitions/can-messages/"
);

#[proc_macro]
pub fn generate_all_messages(_stream: TokenStream) -> TokenStream {
    // get the parsed JSON of each valid spec file
    let __parsed = match fs::read_dir(CANGEN_SPEC_PATH) {
        Ok(__parsed) => __parsed,
        Err(__error) => {
            eprintln!("Could not read from directory: {CANGEN_SPEC_PATH} with error: {__error}");
            return TokenStream::new();
        }
    };

    let __json: Vec<OdysseyMsg> = __parsed
        .filter_map(Result::ok)
        .map(|__entry| __entry.path())
        .filter(|__path| __path.is_file() && __path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|__path| {
            let __data = match fs::read_to_string(__path) {
                Ok(__data) => __data,
                Err(__error) => {
                    eprintln!("Could not read file: {__error}");
                    return None;
                }
            };

            // treat deserialization failures as critical
            Some(
                serde_json::from_str::<Vec<OdysseyMsg>>(&__data)
                    .expect("Error deserializing {__path}"),
            )
        })
        .flatten()
        .collect();

    let decls: Vec<proc_macro2::TokenStream> = __json
        .into_iter()
        .filter_map(|f| match f {
            OdysseyMsg::Can(canmsg) => Some(build_struct(canmsg)),
            OdysseyMsg::Meta(_meta_msg) => None,
        })
        .collect();

    // combine the structs together
    let res = quote! {
        #( #decls )*
    };

    TokenStream::from(res)
}

/// Smallest unsigned Rust integer type able to hold `bits` bits.
fn uint_for(bits: usize) -> proc_macro2::TokenStream {
    match bits {
        0..=8 => quote! { u8 },
        9..=16 => quote! { u16 },
        17..=32 => quote! { u32 },
        _ => quote! { u64 },
    }
}

/// Smallest signed Rust integer type able to hold `bits` bits.
fn int_for(bits: usize) -> proc_macro2::TokenStream {
    match bits {
        8 => quote! { i8 },
        16 => quote! { i16 },
        32 => quote! { i32 },
        64 => quote! { i64 },
        _ => panic!("Invalid not byte aligned signed integer!"),
    }
}

/// Path to a helper in `cangen`'s `conv` module (e.g. `conv::div_from_u16`).
fn conv_path(name: &str) -> proc_macro2::TokenStream {
    let id = Ident::new(name, proc_macro2::Span::call_site());
    quote! { conv::#id }
}

/// Build `#[doc = ..]` attributes for a field from its matched `NetField`.
///
/// A `NetField`'s `values` list the 1-indexed points it documents, so several
/// points (e.g. IMU x/y/z) can share one field.
fn doc_attrs(nf: Option<&NetField>) -> proc_macro2::TokenStream {
    let Some(nf) = nf else { return quote!() };

    let mut lines: Vec<String> = Vec::new();
    let mut push_para = |text: &str| {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if !lines.is_empty() {
            lines.push(String::new()); // blank line between paragraphs
        }
        // Prefix a space so rendered rustdoc reads `/// text`, not `///text`.
        lines.push(format!(" {text}"));
    };

    push_para(&nf.doc);
    if let Some(desc) = nf.desc.as_deref() {
        push_para(desc);
    }
    if !nf.unit.trim().is_empty() {
        push_para(&format!("Units: {}", nf.unit.trim()));
    }

    // bitfield_struct appends it to all relevant functions as well!
    let docs = lines.into_iter().map(|l| quote! { #[doc = #l] });
    quote! { #(#docs)* }
}

/// Build the `bitfield_struct` field declaration for a single CAN point, plus
/// any `try_with_*` / `try_set_*` validating accessors it needs.
///
/// Numeric points are exposed as `f32` (the Rust equivalent of the spec's C
/// `float`) with `#[bits(N, from = .., into = ..)]` pointing at the `conv`
/// helpers, which apply the formatter's divisor/multiplier. Because the `into`
/// helpers *saturate*, the generated `with_*`/`set_*` never panic — so for those
/// scaled fields we also emit `try_*` accessors that reject out-of-range values
/// up front (`bitfield_struct`'s own `*_checked` would always succeed here).
///
/// Plain integers and booleans use `bitfield_struct`'s native support (their
/// `*_checked` already work). Unnamed or `parse: false` points become
/// `_reserved` padding.
///
/// Returns `(field_declaration, checked_accessor_methods)`; the second is empty
/// for non-scaled points.
fn field_tokens(
    i: usize,
    f: &definition_rs::CANPoint,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let bits = proc_macro2::Literal::usize_unsuffixed(f.size);
    let signed = f.signed.unwrap_or(false);

    // A point contributes bits but no accessor when parse=false or no name
    // This means no name is inferred and must be specified for code gen
    let named = f.name.as_ref().filter(|n| !n.is_empty());
    if named.is_none() || !f.parse.unwrap_or(true) {
        // name all reserved bits as such
        let ident = Ident::new(&format!("_reserved{i}"), proc_macro2::Span::call_site());
        let ty = uint_for(f.size);
        return (quote! { #[bits(#bits)] #ident: #ty }, quote!());
    }
    let ident = Ident::new(
        AsSnakeCase(named.unwrap()).0,
        proc_macro2::Span::call_site(),
    );
    let storage = uint_for(f.size).to_string();
    let b = proc_macro2::Literal::u32_unsuffixed(f.size as u32);

    // Scaled `f32` accessor: physical value in/out, raw integer stored. Also
    // emits `try_with_*` / `try_set_*` that reject values that can't be
    // represented in `f.size` bits (before `into` would saturate them).
    let scaled = |op: &str, arg: u32| -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
        let sp = if signed { "s" } else { "" };
        let from_fn = conv_path(&format!("{sp}{op}_from_{storage}"));
        let into_fn = conv_path(&format!("{sp}{op}_into_{storage}"));
        let arg_lit = proc_macro2::Literal::u32_unsuffixed(arg);
        let decl = quote! {
            #[bits(#bits, from = #from_fn::<#b, #arg_lit>, into = #into_fn::<#b, #arg_lit>)]
            pub #ident: f32
        };

        // Representable raw range for this field, as f32 comparison bounds.
        let n = f.size as i32;
        let (lo, hi) = if signed {
            let half = 2f64.powi(n - 1);
            (-half, half - 1.0)
        } else {
            (0.0, 2f64.powi(n) - 1.0)
        };
        let lo = proc_macro2::Literal::f32_suffixed(lo as f32);
        let hi = proc_macro2::Literal::f32_suffixed(hi as f32);
        let arg_f = proc_macro2::Literal::f32_suffixed(arg as f32);
        // raw counts: divide stores `v * ARG`, multiply stores `v / ARG`.
        let raw = if op == "mul" {
            quote! { v / #arg_f }
        } else {
            quote! { v * #arg_f }
        };
        let name = ident.to_string();
        let try_with = quote::format_ident!("try_with_{}", ident);
        let try_set = quote::format_ident!("try_set_{}", ident);
        let with_real = quote::format_ident!("with_{}", ident);
        let set_real = quote::format_ident!("set_{}", ident);
        let checked = quote! {
            #[doc = concat!("Like [`Self::", stringify!(#with_real), "`] but returns")]
            #[doc = "[`OutOfRange`] instead of saturating an unrepresentable value."]
            pub fn #try_with(self, v: f32) -> ::core::result::Result<Self, OutOfRange> {
                let raw = #raw;
                if (#lo..#hi).contains(&raw) {
                    ::core::result::Result::Ok(self.#with_real(v))
                } else {
                    ::core::result::Result::Err(OutOfRange { field: #name })
                }
            }
            #[doc = concat!("Like [`Self::", stringify!(#set_real), "`] but returns")]
            #[doc = "[`OutOfRange`] instead of saturating an unrepresentable value."]
            pub fn #try_set(&mut self, v: f32) -> ::core::result::Result<(), OutOfRange> {
                let raw = #raw;
                if (#lo..#hi).contains(&raw) {
                    self.#set_real(v);
                    ::core::result::Result::Ok(())
                } else {
                    ::core::result::Result::Err(OutOfRange { field: #name })
                }
            }
        };
        (decl, checked)
    };

    // A raw 32-bit IEEE-754 float is stored verbatim (every bit pattern is
    // valid, so no range check is needed).
    if f.ieee754_f32.unwrap_or(false) {
        return (
            quote! {
                #[bits(#bits, from = f32::from_bits, into = f32::to_bits)]
                pub #ident: f32
            },
            quote!(),
        );
    }

    // this turns our human language into the div/multiply functions
    // see scaled! in cangen lib.rs
    if let Some(fmt) = &f.formatter {
        match fmt.key.as_str() {
            "divide" => return scaled("div", fmt.arg as u32),
            "multiply" => return scaled("mul", fmt.arg as u32),
            _ => {}
        }
    }

    // splits up our c_type into 3 sections: float, bool, and uint/ints
    match f.c_type.as_deref() {
        // Unformatted float: identity scaling (divisor 1) so the accessor stays `f32`.
        Some("float") => scaled("div", 1),
        Some("bool") if f.size == 1 => (quote! { #[bits(#bits)] pub #ident: bool }, quote!()),
        _ => {
            let ty = if signed {
                int_for(f.size)
            } else {
                uint_for(f.size)
            };
            (quote! { #[bits(#bits)] pub #ident: #ty }, quote!())
        }
    }
}

/// CANMsg -> bitfield and associated Impls
fn build_struct(msg: CANMsg) -> proc_macro2::TokenStream {
    // the total count of bits sent, including parse=false bits
    let bit_cnt: usize = msg.points.iter().map(|f| f.size).sum();
    let min_size_raw = bit_cnt.div_ceil(8);
    // the minimum number of bytes to hold the message (effectively the DLC)
    let min_size = quote! { #min_size_raw };

    let struct_name = Ident::new(
        &format!(
            "{}",
            AsPascalCase(msg.desc.clone().to_lowercase().replace(' ', "_"))
        ),
        proc_macro2::Span::call_site(),
    );

    let id_int = u32::from_str_radix(msg.id.clone().trim_start_matches("0x"), 16).unwrap();
    let ext_ident = msg.is_ext.unwrap_or(false);

    // this ends up being instantiated at a const context, compile time (prob?)
    let id_decl = if ext_ident {
        quote! { Id::Extended(ExtendedId::new(#id_int).unwrap()) }
    } else {
        // StandardId::new takes a u16; standard IDs always fit.
        quote! { Id::Standard(StandardId::new(#id_int as u16).unwrap()) }
    };

    let ts_bits = match bit_cnt {
        0..=8 => 8usize,
        9..=16 => 16,
        17..=32 => 32,
        _ => 64,
    };

    // Map each 1-indexed point position to the `NetField` that documents it.
    // A field's `values` may list several points (e.g. IMU x/y/z share a doc).
    let mut doc_for: std::collections::HashMap<usize, &NetField> = std::collections::HashMap::new();
    for nf in &msg.fields {
        for &v in &nf.values {
            doc_for.entry(v).or_insert(nf);
        }
    }

    let mut field_declarations: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut checked_methods: Vec<proc_macro2::TokenStream> = Vec::new();
    for (i, f) in msg.points.iter().enumerate() {
        let docs = doc_attrs(doc_for.get(&(i + 1)).copied());
        let (field, checked) = field_tokens(i, f);
        field_declarations.push(quote! { #docs #field });
        checked_methods.push(checked);
    }

    // Fill the remainder of the backing integer with trailing padding.
    if ts_bits > bit_cnt {
        let pad = ts_bits - bit_cnt;
        let bits = proc_macro2::Literal::usize_unsuffixed(pad);
        let ty = uint_for(pad);
        let ident = Ident::new(
            &format!("_reserved{}", msg.points.len()),
            proc_macro2::Span::call_site(),
        );
        field_declarations.push(quote! { #[bits(#bits)] #ident: #ty });
    }

    let ts = uint_for(bit_cnt);

    // Generate the final output Rust code
    let expanded = quote! {
        #[bitfield(#ts)]
        pub struct #struct_name {
            #(#field_declarations),*
        }
    };

    let tr = quote! {
        impl ToCanFrame for #struct_name {
            type Repr  = #ts;
            const LEN: usize = #min_size;
            const ID: Id = #id_decl;
        }
    };

    // Range-validating accessors for the saturating (scaled `f32`) fields.
    let checked = quote! {
        impl #struct_name {
            #(#checked_methods)*
        }
    };

    quote! {
        #tr
        #expanded
        #checked
    }
}

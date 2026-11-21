//! Code generation for `m24xx-ner`. Both macros are re-exported from `m24xx_ner`,
//! which is also what the generated code refers to.
//!
//! A macro sees tokens, not types, so neither `M24C32`'s capacity nor a field's page
//! count can be read here. Both come back through the type system, as consts the
//! compiler folds.
//!
//! Nothing here knows which schemes exist: a field's declared type is passed through
//! verbatim, so a new scheme is a type in the driver and no change at all here.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Fields, Ident, ItemStruct, parse_macro_input};

/// Where the generated code looks for everything it needs.
fn krate() -> TokenStream2 {
    quote!(::m24xx_ner)
}

fn emit_layout(part: &Ident, item: &ItemStruct) -> syn::Result<TokenStream2> {
    let m = krate();
    let store = &item.ident;
    let vis = &item.vis;

    let Fields::Named(fields) = &item.fields else {
        return Err(syn::Error::new_spanned(
            store,
            "a layout needs named fields",
        ));
    };
    if fields.named.is_empty() {
        return Err(syn::Error::new_spanned(
            store,
            "a layout needs at least one field",
        ));
    }
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "a layout declares its own generics",
        ));
    }

    // Offsets must not depend on `I2C` or `D`, or they could not be const arguments
    // on the accessors, so they live in a module of their own.
    let pages_mod = format_ident!("__{}_pages", store);

    let mut offsets = Vec::new();
    let mut checks = Vec::new();
    let mut state_fields = Vec::new();
    let mut state_inits = Vec::new();
    let mut accessors = Vec::new();
    let mut updates = Vec::new();
    let mut map = Vec::new();
    let page = quote!(#part::<(), ()>::page_size());

    // Built in the module's own scope, where each offset sees the one before it.
    let mut running = quote!(0usize);
    for (index, field) in fields.named.iter().enumerate() {
        let name = field.ident.as_ref().expect("named fields");
        let ty = &field.ty;
        let attrs = &field.attrs;

        // Positional rather than named after the field: a duplicate field should
        // report itself, not a clash between two consts nobody wrote.
        let konst = format_ident!("__P{}", index);

        // Spanned on the field's type, which is both what failed and what to change.
        checks.push(quote_spanned!(ty.span()=>
            const _: () = assert!(
                #m::__private::fits(<#ty as #m::MemoryScheme>::RECORD_BYTES, #page),
                "this field does not fit one page of this part"
            );
        ));

        offsets.push(quote!( pub const #konst: usize = #running; ));
        running = quote!( #konst + <#ty as #m::MemoryScheme>::PAGES );
        map.push(format!("- `{name}`"));

        let first = quote!(#pages_mod::#konst);
        state_fields.push(quote!( #name: #m::Field<#ty>, ));
        state_inits.push(quote!( #name: ::core::default::Default::default(), ));
        updates.push(quote!( store.#name().update().await?; ));
        accessors.push(quote! {
            #(#attrs)*
            pub fn #name(&mut self) -> #m::Access<'_, #part<I2C, D>, #ty, { #page }, { #first }> {
                #m::Access::__new(&mut self.eeprom, &mut self.#name)
            }
        });
    }

    let total = running;
    let map = map.join("\n");
    let doc = format!(
        "Everything on one part, and all that can reach it once attached.\n\n\
         Fields, in the order they take pages:\n\n{map}"
    );

    Ok(quote! {
        #(#checks)*

        #[doc(hidden)]
        #[allow(non_snake_case, unused_imports)]
        mod #pages_mod {
            use super::*;
            #(#offsets)*
            pub const TOTAL: usize = #total;
        }

        const _: () = assert!(
            #m::__private::fits(#pages_mod::TOTAL, #part::<(), ()>::pages()),
            "layout does not fit the part"
        );

        #[doc = #doc]
        #[allow(dead_code)]
        #vis struct #store<I2C, D> {
            eeprom: #part<I2C, D>,
            #(#state_fields)*
        }

        impl<I2C, D> #m::EepromLayout for #store<I2C, D>
        where
            I2C: #m::__private::I2c,
            D: #m::__private::DelayNs,
        {
            type Part = #part<I2C, D>;
            const PAGES: usize = #pages_mod::TOTAL;

            async fn attach(part: Self::Part) -> Result<Self, #m::Error<I2C::Error>> {
                let mut store = Self {
                    eeprom: part,
                    #(#state_inits)*
                };
                #(#updates)*
                Ok(store)
            }
        }

        #[allow(dead_code)]
        impl<I2C, D> #store<I2C, D>
        where
            I2C: #m::__private::I2c,
            D: #m::__private::DelayNs,
        {
            #(#accessors)*
        }
    })
}

// entry points

/// Declare what lives on one part. See `m24xx_ner::eeprom_layout`.
#[proc_macro_attribute]
pub fn eeprom_layout(attr: TokenStream, item: TokenStream) -> TokenStream {
    let part = parse_macro_input!(attr as Ident);
    let item = parse_macro_input!(item as ItemStruct);
    emit_layout(&part, &item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

// derive

/// Derive `Storable` for a struct whose every field is itself a `Storable`. Offsets
/// are the running sum of the fields before them; the work is delegated to each field.
#[proc_macro_derive(Storable)]
pub fn derive_storable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    emit_derive(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn emit_derive(input: &syn::DeriveInput) -> syn::Result<TokenStream2> {
    let m = krate();
    let name = &input.ident;

    let syn::Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            name,
            "Storable can only be derived for a struct",
        ));
    };
    let syn::Fields::Named(named) = &data.fields else {
        return Err(syn::Error::new_spanned(
            name,
            "Storable needs named fields, so the stored order is the declared order",
        ));
    };
    if named.named.is_empty() {
        return Err(syn::Error::new_spanned(
            name,
            "a storable value needs at least one field",
        ));
    }

    let mut lens = Vec::new();
    let mut encodes = Vec::new();
    let mut decodes = Vec::new();
    let mut offset = quote!(0usize);

    for field in &named.named {
        let f = field.ident.as_ref().expect("named");
        let ty = &field.ty;
        let span = ty.span();

        lens.push(quote_spanned!(span=> + <#ty as #m::Storable>::LEN));
        encodes.push(quote_spanned!(span=>
            <#ty as #m::Storable>::encode(
                &self.#f,
                &mut buf[#offset..][..<#ty as #m::Storable>::LEN],
            );
        ));
        decodes.push(quote_spanned!(span=>
            #f: <#ty as #m::Storable>::decode(&buf[#offset..][..<#ty as #m::Storable>::LEN])?,
        ));
        offset = quote!(#offset + <#ty as #m::Storable>::LEN);
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #m::Storable for #name #ty_generics #where_clause {
            const LEN: usize = 0usize #(#lens)*;

            fn encode(&self, buf: &mut [u8]) {
                #(#encodes)*
            }

            fn decode(buf: &[u8]) -> Option<Self> {
                Some(Self { #(#decodes)* })
            }
        }
    })
}

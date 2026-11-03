//! Code generation for `m24xx-ner`. Both macros are re-exported from `m24xx_ner`,
//! which is also what the generated code refers to.
//!
//! A macro sees tokens, not types, so it cannot resolve `M24C32` to a capacity: the
//! fit checks are `const assert!`s it emits, spanned on the offending region so the
//! error lands on the declaration.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Ident, LitInt, Token, Type, parse_macro_input};

/// Pages one double buffered value occupies.
const DOUBLE_SLOTS: usize = 2;

/// Where the generated code looks for everything it needs.
fn krate() -> TokenStream2 {
    quote!(::m24xx_ner)
}

// parsing

/// A named run of pages.
// A handful of these exist for the length of one expansion, so the size gap
// between a parsed type and a page count is not worth boxing.
#[allow(clippy::large_enum_variant)]
enum Region {
    /// Two slots holding a value the caller has made `Storable`.
    Buffered {
        attrs: Vec<Attribute>,
        name: Ident,
        record: Type,
    },
    /// Pages with no scheme over them.
    Raw {
        attrs: Vec<Attribute>,
        name: Ident,
        pages: usize,
    },
}

impl Region {
    fn pages(&self) -> usize {
        match self {
            Self::Buffered { .. } => DOUBLE_SLOTS,
            Self::Raw { pages, .. } => *pages,
        }
    }

    fn name(&self) -> &Ident {
        match self {
            Self::Buffered { name, .. } | Self::Raw { name, .. } => name,
        }
    }
}

impl Parse for Region {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let kind: Ident = input.parse()?;

        if kind == "Raw" {
            input.parse::<Token![<]>()?;
            let lit: LitInt = input.parse()?;
            input.parse::<Token![>]>()?;
            let pages = lit.base10_parse::<usize>()?;
            if pages == 0usize {
                return Err(syn::Error::new(
                    lit.span(),
                    "a region of no pages is useless",
                ));
            }
            return Ok(Self::Raw { attrs, name, pages });
        }

        if kind != "DoubleBuffered" {
            return Err(syn::Error::new(
                kind.span(),
                "expected `DoubleBuffered<T>` or `Raw<N>`",
            ));
        }
        input.parse::<Token![<]>()?;
        let record: Type = input.parse()?;
        input.parse::<Token![>]>()?;

        Ok(Self::Buffered {
            attrs,
            name,
            record,
        })
    }
}

/// Everything declared for one chip.
struct Layout {
    store: Ident,
    part: Ident,
    regions: Vec<Region>,
}

impl Parse for Layout {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let store: Ident = input.parse()?;
        input.parse::<Token![for]>()?;
        let part: Ident = input.parse()?;
        input.parse::<Token![;]>()?;

        let mut regions = Vec::new();
        while !input.is_empty() {
            regions.push(input.parse::<Region>()?);
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        if regions.is_empty() {
            return Err(syn::Error::new(
                store.span(),
                "a layout needs at least one region",
            ));
        }
        reject_duplicates(regions.iter().map(Region::name), "region")?;

        Ok(Self {
            store,
            part,
            regions,
        })
    }
}

/// Two regions on one part, or two fields in one record, sharing a name is a typo
/// worth naming rather than letting it surface as a duplicate struct member.
fn reject_duplicates<'a>(names: impl Iterator<Item = &'a Ident>, what: &str) -> syn::Result<()> {
    let mut seen: Vec<&Ident> = Vec::new();
    for name in names {
        if seen.contains(&name) {
            return Err(syn::Error::new(
                name.span(),
                format!("duplicate {what} `{name}`"),
            ));
        }
        seen.push(name);
    }
    Ok(())
}

// emitting

fn emit_layout(layout: &Layout) -> syn::Result<TokenStream2> {
    let m = krate();
    let Layout {
        store,
        part,
        regions,
    } = layout;

    let mut checks = Vec::new();
    let mut slot_fields = Vec::new();
    let mut slot_inits = Vec::new();
    let mut accessors = Vec::new();
    let mut opens = Vec::new();

    // The running sum that a declarative macro has to thread through every rule.
    let mut page = 0usize;
    for region in regions {
        let first = page;
        page += region.pages();

        match region {
            Region::Buffered {
                attrs,
                name,
                record,
            } => {
                // Asked for plainly, so a type that is not storable says so rather
                // than surfacing later as a missing method on the handle.
                checks.push(quote_spanned!(record.span()=>
                    const _: fn() = || {
                        fn assert_storable<T: #m::Storable>() {}
                        assert_storable::<#record>();
                    };
                ));
                checks.push(quote_spanned!(record.span()=>
                    const _: () = assert!(
                        <#record as #m::Storable>::LEN + #m::__private::HEADER
                            <= #part::<(), ()>::page_size(),
                        "value does not fit in one page of this part"
                    );
                ));

                slot_fields.push(quote!( #name: #m::SlotState<#record>, ));
                slot_inits.push(quote!( #name: #m::SlotState::new(), ));
                opens.push(quote!( self.#name().open().await?; ));
                accessors.push(quote! {
                    #(#attrs)*
                    pub fn #name(
                        &mut self,
                    ) -> #m::DoubleBuffered<
                        '_, #part<I2C, D>, #record,
                        { #part::<(), ()>::page_size() }, #first
                    > {
                        #m::DoubleBuffered::__new(&mut self.eeprom, &mut self.#name)
                    }
                });
            }
            Region::Raw { attrs, name, pages } => {
                accessors.push(quote! {
                    #(#attrs)*
                    pub fn #name(
                        &mut self,
                    ) -> #m::Raw<
                        '_, #part<I2C, D>,
                        { #part::<(), ()>::page_size() }, #first, #pages
                    > {
                        #m::Raw::__new(&mut self.eeprom)
                    }
                });
            }
        }
    }

    let total = page;
    let map = regions
        .iter()
        .scan(0usize, |at, r| {
            let first = *at;
            *at += r.pages();
            Some(format!("- `{}`: pages {}..{}", r.name(), first, *at))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let doc = format!("Everything on one part. The only way to reach it.\n\n{map}");

    // Spanned on the store, the one place that stands for the layout as a whole.
    let fits = quote_spanned!(store.span()=>
        const _: () = assert!(
            #total <= #part::<(), ()>::pages(),
            "layout does not fit the part"
        );
    );

    Ok(quote! {
        #(#checks)*

        #fits

        #[doc = #doc]
        #[allow(dead_code)]
        pub struct #store<I2C, D> {
            eeprom: #part<I2C, D>,
            #(#slot_fields)*
        }

        #[allow(dead_code)]
        impl<I2C, D> #store<I2C, D> {
            /// Pages this layout occupies.
            pub const PAGES: usize = #total;

            /// Bind this layout to the part strapped to `chip_enable`. Reads
            /// nothing yet; call [`open`](Self::open) once the executor is up.
            pub fn init(i2c: I2C, delay: D, chip_enable: #m::ChipEnable) -> Self {
                Self {
                    eeprom: <#part<I2C, D>>::__new(i2c, delay, chip_enable),
                    #(#slot_inits)*
                }
            }
        }

        #[allow(dead_code)]
        impl<I2C, D> #store<I2C, D>
        where
            I2C: #m::__private::I2c,
            D: #m::__private::DelayNs,
        {
            /// Read every double buffered region. Call once, at startup.
            pub async fn open(&mut self) -> Result<(), #m::StoreError<#m::Error<I2C::Error>>> {
                #(#opens)*
                Ok(())
            }

            #(#accessors)*
        }
    })
}

// entry points

/// Declare what lives on one part. See `m24xx_ner::eeprom_layout`.
#[proc_macro]
pub fn eeprom_layout(input: TokenStream) -> TokenStream {
    let layout = parse_macro_input!(input as Layout);
    emit_layout(&layout)
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

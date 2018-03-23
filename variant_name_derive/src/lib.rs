extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::DeriveInput;

#[proc_macro_derive(VariantName)]
pub fn variant_name(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input: DeriveInput = syn::parse(input).unwrap();

    let name = &input.ident;

    // Build the output, possibly using quasi-quotation
    let expanded = match input.data {
        syn::Data::Struct(_) => {
            quote! {
                impl VariantName for #name {
                    fn variant_name(&self) -> &'static str {
                        stringify!(#name)
                    }
                }
            }
        }
        syn::Data::Enum(enu) => {
            let variants1 = enu.variants.iter().map(|v| v.ident);
            let variants2 = enu.variants.iter().map(|v| v.ident);
            quote! {
                impl VariantName for #name {
                    fn variant_name(&self) -> &'static str {
                        match self {
                            #(#variants1 => stringify!(#variants2)),*
                        }
                    }
                }
            }
        }
        syn::Data::Union(_) => panic!("VariantName can only be derived for Enum or Struct!"),
    };

    // Hand the output tokens back to the compiler
    expanded.into()
}

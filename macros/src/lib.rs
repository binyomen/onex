use {
    proc_macro::TokenStream,
    quote::{quote, quote_spanned},
    syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Field, Fields, Ident},
};

struct Variant {
    name: Ident,
    field: Field,
}

#[proc_macro_derive(ErrorEnum)]
pub fn derive_error_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let variants = match input.data {
        Data::Enum(data) => data
            .variants
            .into_iter()
            .map(|v| {
                let field = match v.fields {
                    Fields::Unnamed(u) => u.unnamed.into_iter().next().unwrap(),
                    _ => unimplemented!(),
                };
                Variant {
                    name: v.ident,
                    field,
                }
            })
            .collect::<Vec<Variant>>(),
        _ => unimplemented!(),
    };

    let display_impl = make_display_impl(&name, &variants);
    let error_impl = make_error_impl(&name, &variants);
    let from_impls = make_from_impls(&name, &variants);

    TokenStream::from(quote! {
        #display_impl
        #error_impl
        #from_impls
    })
}

fn make_display_impl(name: &Ident, variants: &[Variant]) -> proc_macro2::TokenStream {
    let cases = variants.iter().map(|v| {
        let v_name = &v.name;
        let v_field = &v.field;
        quote_spanned! { v_field.span() =>
            #name::#v_name(err) => err.fmt(f),
        }
    });
    quote! {
        impl fmt::Display for #name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    #(#cases)*
                }
            }
        }
    }
}

fn make_error_impl(name: &Ident, variants: &[Variant]) -> proc_macro2::TokenStream {
    let cases = variants.iter().map(|v| {
        let v_name = &v.name;
        let v_field = &v.field;
        quote_spanned! { v_field.span() =>
            #name::#v_name(err) => Some(err),
        }
    });
    quote! {
        impl error::Error for #name {
            fn source(&self) -> Option<&(dyn error::Error + 'static)> {
                match self {
                    #(#cases)*
                }
            }
        }
    }
}

fn make_from_impls(name: &Ident, variants: &[Variant]) -> proc_macro2::TokenStream {
    let impls = variants.iter().map(|v| {
        let v_name = &v.name;
        let v_field = &v.field;
        let v_ty = &v_field.ty;
        quote_spanned! { v_field.span() =>
            impl From<#v_ty> for #name {
                fn from(err: #v_ty) -> Self {
                    #name::#v_name(err)
                }
            }
        }
    });
    quote! {
        #(#impls)*
    }
}

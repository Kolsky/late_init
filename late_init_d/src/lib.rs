use proc_macro::TokenStream;
use quote::*;
use syn::Data;
use core::iter::*;

struct LifetimesIdents<LifetimeIter, IdentIter> {
    lifetimes: LifetimeIter,
    idents: IdentIter,
}

impl<
        'a,
        LifetimeIter: Iterator<Item = &'a syn::Lifetime> + Clone,
        IdentIter: Iterator<Item = &'a syn::Ident> + Clone,
    > ToTokens for LifetimesIdents<LifetimeIter, IdentIter>
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let punct = <syn::Token![,]>::default();
        for lifetime in self.lifetimes.clone() {
            lifetime.to_tokens(tokens);
            punct.to_tokens(tokens);
        }
        for ident in self.idents.clone() {
            ident.to_tokens(tokens);
            punct.to_tokens(tokens);
        }
    }
}

#[proc_macro_derive(LateInit)]
pub fn late_init(input: TokenStream) -> TokenStream {
    let syn::DeriveInput {
        ident,
        mut generics,
        data,
        ..
    } = syn::parse_macro_input!(input);
    let data = match data {
        Data::Struct(s) => s,
        Data::Enum(_) => panic!("enums are not supported"),
        Data::Union(_) => panic!("unions are not supported"),
    };
    generics.type_params_mut().for_each(|tp| {
        tp.eq_token = None;
        tp.default = None;
    });
    let lifetimes: Vec<_> = generics.lifetimes().map(|l| l.lifetime.clone()).collect();
    let types = generics.type_params().map(|tp| tp.ident.clone());
    let consts = generics.const_params().map(|cp| cp.ident.clone());
    let types_consts: Vec<_> = types.chain(consts).collect();
    let late_init_ident = format_ident!("{}LateInit", ident);
    let late_init_ident_mod = mod_name(&ident);
    let mut late_init_generics = generics.clone();
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let false_ident = format_ident!("false");
    let true_ident = format_ident!("true");
    let late_init_consts_default = repeat(&false_ident).take(data.fields.len());
    let late_init_type_params_default = LifetimesIdents {
        lifetimes: lifetimes.iter(),
        idents: types_consts.iter().chain(late_init_consts_default),
    };
    let mut late_init_consts = vec![];
    let mut init = vec![];
    data.fields.iter().enumerate().for_each(|(i, f)| {
        let name = f.ident.as_ref().map_or_else(
            || format_ident!("_{}Init", i),
            |ident| format_ident!("{}Init", ident),
        );
        let init_field = auto_init(&name, f.ident.as_ref(), i);
        late_init_generics
            .params
            .push(syn::parse_quote!(const #name: bool));
        late_init_consts.push(name);
        init.push(init_field);
    });
    let late_init_type_params = LifetimesIdents {
        lifetimes: lifetimes.iter(),
        idents: types_consts.iter().chain(late_init_consts.iter()),
    };
    let fns: Vec<_> = data.fields.iter().enumerate().map(|(i, f)| {
        let (cl, rem) = late_init_consts.split_at(i);
        let (ci, cr) = rem.split_first().unwrap();
        let fn_type_params = LifetimesIdents {
            lifetimes: lifetimes.iter(),
            idents: types_consts.iter().chain(cl).chain(once(&true_ident)).chain(cr),
        };
        let name = f.ident.clone().unwrap_or_else(|| format_ident!("set_{}", i));
        let init_field = val_init(f.ident.as_ref(), i);
        let ty = &f.ty;
        quote! {
            #[must_use]
            pub fn #name(mut self, val: #ty) -> #late_init_ident<#fn_type_params>
            where
                InitSt<#ty, #ci>: Uninit,
            {
                let t = self.0.as_mut_ptr();
                #init_field
                #late_init_ident(self.0)
            }
        }
    })
    .collect();
    let field_types = data.fields.iter().map(|f| &f.ty);

    let output = quote! {
        mod #late_init_ident_mod
        {
            use super::*;
            use late_init::markers::{InitSt, Uninit, AutoInit};
            #[allow(non_upper_case_globals)]
            pub(in super) struct #late_init_ident #late_init_generics(::core::mem::MaybeUninit<#ident #type_generics>) #where_clause;

            impl #impl_generics ::core::default::Default for #late_init_ident<#late_init_type_params_default> #where_clause {
                fn default() -> Self {
                    Self(::core::mem::MaybeUninit::uninit())
                }
            }

            #[allow(non_upper_case_globals)]
            impl #late_init_generics #late_init_ident<#late_init_type_params> #where_clause {
                #(#fns)*

                pub fn finish(mut self) -> #ident #type_generics
                where
                    #(InitSt<#field_types, #late_init_consts>: AutoInit,)*
                {
                    let t = self.0.as_mut_ptr();
                    #(#init)*
                    unsafe { self.0.assume_init() }
                }
            }
        }
        use #late_init_ident_mod::#late_init_ident;
    };
    output.into()
}

fn mod_name(derive_ident: &syn::Ident) -> syn::Ident {
    let guid = "9c7ea5a7_03be_4f1c_a760_c570173fc90f";
    let ident = derive_ident.to_string();
    let base32: String = ident
        .as_bytes()
        .chunks(5)
        .flat_map(|bytes| {
            let mut bits = [0; 8];
            bits[..bytes.len()].copy_from_slice(bytes);
            let mut bits = u64::from_le_bytes(bits).wrapping_shl(5);
            let count = (8 * bytes.len() + 4) / 5;
            (0..count).map(move |_| {
                bits >>= 5;
                (match (bits as u8) & 0b11111 {
                    b @ 10.. => b'a' + (b - 10),
                    b @ 0.. => b'0' + b,
                }) as char
            })
        })
        .collect();
    format_ident!("mod_{}_{}", guid, base32)
}

fn val_init(field_name: Option<&syn::Ident>, field_index: usize) -> syn::Expr {
    fn val_init_any(field: impl ToTokens) -> syn::Expr {
        syn::parse_quote!(unsafe {
            ::core::ptr::write(::core::ptr::addr_of_mut!((*t).#field), val);
        })
    }
    field_name.map_or_else(|| val_init_any(syn::Index::from(field_index)), val_init_any)
}

fn auto_init(
    const_name: &syn::Ident,
    field_name: Option<&syn::Ident>,
    field_index: usize,
) -> syn::Expr {
    fn auto_init_any(field: impl ToTokens, const_name: &syn::Ident) -> syn::Expr {
        syn::parse_quote!(unsafe {
            let t = ::core::ptr::addr_of_mut!((*t).#field);
            InitSt::<_, #const_name>(t).init();
        })
    }
    field_name.map_or_else(
        || auto_init_any(syn::Index::from(field_index), const_name),
        |field| auto_init_any(field, const_name),
    )
}
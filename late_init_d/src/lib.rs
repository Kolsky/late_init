use proc_macro::TokenStream;
use quote::*;
use syn::Data;

#[proc_macro_derive(LateInit)]
pub fn late_init(input: TokenStream) -> TokenStream {
    let syn::DeriveInput {
        ident,
        generics,
        data,
        ..
    } = syn::parse_macro_input!(input);
    let data = match data {
        Data::Struct(s) => s,
        Data::Enum(_) => panic!("enums are not supported"),
        Data::Union(_) => panic!("unions are not supported"),
    };
    let late_init_ident = format_ident!("{}LateInit", ident);
    let late_init_ident_mod = format_ident!("{}Mod", late_init_ident);
    let mut late_init_generics = generics.clone();
    let late_init_consts_default: Vec<syn::LitBool> =
        vec![syn::parse_quote!(false); data.fields.len()];
    let lifetimes: Vec<_> = generics.lifetimes().map(|l| l.lifetime.clone()).collect();
    let types: Vec<_> = generics.type_params().map(|tp| tp.ident.clone()).collect();
    let mut late_init_consts = vec![];
    let mut init: Vec<syn::Expr> = vec![];
    data.fields.iter().enumerate().for_each(|(i, f)| {
        let name = f.ident.as_ref().map_or_else(
            || format_ident!("_{}Init", i),
            |ident| format_ident!("{}Init", ident),
        );
        let i = syn::Index::from(i);
        let init_field = f.ident.as_ref().map_or_else(
            || {
                syn::parse_quote!(unsafe {
                    let t = ::core::ptr::addr_of_mut!((*t).#i) as *mut _;
                    InitSt::<_, #name>(::core::marker::PhantomData).init(t);
                })
            },
            |ident| {
                syn::parse_quote!(unsafe {
                    let t = ::core::ptr::addr_of_mut!((*t).#ident) as *mut _;
                    InitSt::<_, #name>(::core::marker::PhantomData).init(t);
                })
            },
        );
        late_init_generics
            .params
            .push(syn::parse_quote!(const #name: bool));
        late_init_consts.push(name);
        init.push(init_field);
    });
    let fns: Vec<_> = data.fields.iter().enumerate().map(|(i, f)| {
        let (cl, rem) = late_init_consts.split_at(i);
        let (ci, cr) = rem.split_first().unwrap();
        let i = syn::Index::from(i);
        let name = f.ident.as_ref().map_or_else(
            || format_ident!("set_{}", i),
            |ident| ident.clone(),
        );
        let init_field: syn::Expr = f.ident.as_ref().map_or_else(
            || {
                syn::parse_quote!(unsafe {
                    ::core::ptr::write(::core::ptr::addr_of_mut!((*t).#i), val);
                })
            },
            |ident| {
                syn::parse_quote!(unsafe {
                    ::core::ptr::write(::core::ptr::addr_of_mut!((*t).#ident), val);
                })
            }
        );
        let ty = &f.ty;
        quote! {
            pub fn #name(mut self, val: #ty) -> #late_init_ident<#(#lifetimes,)* #(#types,)* #(#cl,)* true, #(#cr,)*>
            where
                late_init::markers::InitSt<#ty, #ci>: late_init::markers::Uninit,
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
        #[allow(non_snake_case)]
        mod #late_init_ident_mod
        {
            use super::*;
            #[allow(non_upper_case_globals)]
            pub(in super) struct #late_init_ident #late_init_generics(::core::mem::MaybeUninit<#ident #generics>);

            impl #generics ::core::default::Default for #late_init_ident<#(#lifetimes,)* #(#types,)* #(#late_init_consts_default,)*> {
                fn default() -> Self {
                    Self(::core::mem::MaybeUninit::uninit())
                }
            }

            #[allow(non_upper_case_globals)]
            impl #late_init_generics #late_init_ident<#(#lifetimes,)* #(#types,)* #(#late_init_consts,)*> {
                #(#fns)*

                pub fn finish(mut self) -> #ident #generics
                where
                    #(late_init::markers::InitSt<#field_types, #late_init_consts>: late_init::markers::AutoInit,)*
                {
                    use late_init::markers::*;
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

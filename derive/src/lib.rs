use proc_macro::TokenStream;
use quote::format_ident;
use syn::{parse_quote, spanned::Spanned};

#[proc_macro_derive(RealQuickSer)]
pub fn derive_quick_ser(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let name = &input.ident;

    let mut generics = input.generics.clone();

    for param in generics.type_params_mut() {
        param.bounds.push(parse_quote!(RealQuickSer));
    }

    let mut type_params = input.generics.clone();
    for param in type_params.type_params_mut() {
        param.bounds.clear();
    }

    quote::quote! {
        impl #generics RealQuickSer for #name #type_params {}
    }.into()
}

#[proc_macro_derive(QuickSer)]
pub fn derive_ser(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let name = &input.ident;

    let mut generics = input.generics.clone();

    for param in generics.type_params_mut() {
        param.bounds.push(parse_quote!(QuickSer));
    }

    let mut type_params = input.generics.clone();
    for param in type_params.type_params_mut() {
        param.bounds.clear();
    }

    let ser = match &input.data {
        syn::Data::Struct(s) => {
            let is_tuple = s.fields.iter().next().map(|f| f.ident.is_none()).unwrap_or(false);
            if is_tuple {
                let names = (0..s.fields.len()).map(syn::Index::from);

                let calls = s.fields.iter().zip(names).map(|(f, i)| {
                    let span = f.ty.span();
                    quote::quote_spanned! {span=>
                        QuickSer::ser(&self.#i, buffer)
                    }
                });
                
                quote::quote! {
                    fn ser(&self, buffer: &mut Vec<u8>) {
                        #(
                            #calls;
                        )*
                    }
                }
            } else {
                let calls = s.fields.iter().map(|f| {
                    let span = f.ty.span();
                    let ident = f.ident.as_ref().unwrap();
                    quote::quote_spanned! {span=>
                        QuickSer::ser(&self.#ident, buffer)
                    }
                });
                
                quote::quote! {
                    fn ser(&self, buffer: &mut Vec<u8>) {
                        #(
                            #calls;
                        )*
                    }
                }
            }
        },
        syn::Data::Enum(e) => {
            let variants = e.variants.iter().enumerate().map(|(i, v)| {
                let ident = &v.ident;
                let is_tuple = v.fields.iter().next().map(|f| f.ident.is_none()).unwrap_or(false);
                let index = i as u8;

                if is_tuple {
                    let fields = (0..v.fields.len()).map(|i| format_ident!("field{}", i));
                    let fields2 = fields.clone();
                    
                    quote::quote!(
                        #name::#ident( #( #fields ),* ) => {
                            QuickSer::ser(&#index, buffer);
                            #(
                                QuickSer::ser(#fields2, buffer);
                            )*
                        }
                    )
                } else {
                    let fields = v.fields.iter().map(|f| f.ident.as_ref().unwrap());
                    let fields2 = fields.clone();
                    
                    quote::quote!(
                        #name::#ident { #( #fields ),* } => {
                            #index.ser(buffer);
                            #(
                                QuickSer::ser(#fields2, buffer);
                            )*
                        }
                    )
                }
            });

            quote::quote! {
                fn ser(&self, buffer: &mut Vec<u8>) {
                    match self {
                        #( #variants )*
                    }
                }
            }
        },
        syn::Data::Union(_) => panic!("union is not supported"),
    };

    let de_ser = match &input.data {
        syn::Data::Struct(s) => {
            let is_tuple = s.fields.iter().next().map(|f| f.ident.is_none()).unwrap_or(false);
            if is_tuple {
                let names = std::iter::repeat(format_ident!("QuickSer")).take(s.fields.len());

                quote::quote! {
                    fn de_ser(progress: &mut usize, buffer: &[u8]) -> Self {
                        Self(#(
                            #names::de_ser(progress, buffer),
                        )*)
                    }
                }
            } else {
                let names = s.fields.iter().map(|f| f.ident.as_ref().unwrap());
                
                quote::quote! {
                    fn de_ser(progress: &mut usize, buffer: &[u8]) -> Self {
                        Self {#(
                            #names: QuickSer::de_ser(progress, buffer),
                        )*}
                    }
                }
            }
        },
        syn::Data::Enum(e) => {
            let variants = e.variants.iter().enumerate().map(|(i, v)| {
                let ident = &v.ident;
                let is_tuple = v.fields.iter().next().map(|f| f.ident.is_none()).unwrap_or(false);
                let index = i as u8;

                if is_tuple {
                    let fields = v.fields.iter().map(|f| {
                        let span = f.ty.span();
                        quote::quote_spanned!(span =>
                            QuickSer::de_ser(progress, buffer)
                        )
                    });
                    
                    quote::quote!(
                        #index => {
                            #name::#ident(#(
                                #fields,
                            )*)
                        }
                    )
                } else {
                    let fields = v.fields.iter().map(|f| { 
                        let span = f.ty.span();
                        let ident = f.ident.as_ref().unwrap();
                        let call = quote::quote_spanned!(span =>
                            QuickSer::de_ser(progress, buffer)
                        );

                        quote::quote_spanned!(span =>
                            #ident: #call
                        )
                    });
                    
                    quote::quote!(
                        #index => {
                            #name::#ident {#(
                                #fields,
                            )*}
                        }
                    )
                }
            });

            quote::quote! {
                fn de_ser(progress: &mut usize, buffer: &[u8]) -> Self {
                    match QuickSer::de_ser(progress, buffer) {
                        #( #variants )*
                        v => panic!("invalid variant {:?}", v),
                    }
                }
            }
        },
        syn::Data::Union(_) => panic!("union is not supported"),
    };

    quote::quote! {
        impl #generics QuickSer for #name #type_params {
            #ser
            #de_ser
        }
    }.into()
}

#[proc_macro_derive(QuickEnumGets)]
pub fn derive_enum_getters(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let name = &input.ident;

    let generics = input.generics.clone();

    let mut type_params = input.generics.clone();
    for param in type_params.type_params_mut() {
        param.bounds.clear();
    }

    let data = match input.data {
        syn::Data::Enum(data) => data,
        _ => panic!("macro only supports enums"),
    };

    let functions = data.variants.iter().map(|v| {
        let ident = &v.ident;
        
        let pascal_case = pascal_to_snake(&ident.to_string());

        let getter_name = format_ident!("{}", pascal_case);
        let mut_getter_name = format_ident!("{}_mut", pascal_case);
        let into_name = format_ident!("into_{}", pascal_case); 

        let is_tuple = v.fields.iter().next().map(|f| f.ident.is_none()).unwrap_or(false);

        if is_tuple {
            let names1 = (0..v.fields.len()).map(|i| format_ident!("field{}", i));
            let names2 = names1.clone();
            let names3 = names1.clone();
            let names4 = names1.clone();
            let names5 = names1.clone();
            let names6 = names1.clone();

            let types1 = v.fields.iter().map(|f| &f.ty);
            let types2 = types1.clone();
            let types3 = types1.clone();
            
            
            quote::quote! {
                pub fn #getter_name(&self) -> (#( &#types1 ),*) {
                    match self {
                        Self::#ident(#(#names1),*) => (#(#names2),*),
                        var => panic!("invalid variant {:?}", var),
                    }
                }

                pub fn #mut_getter_name(&mut self) -> (#( &mut #types2 ),*) {
                    match self {
                        Self::#ident(#(#names3),*) => (#(#names4),*),
                        var => panic!("invalid variant {:?}", var),
                    }
                }

                pub fn #into_name(self) -> (#(#types3),*) {
                    match self {
                        Self::#ident(#(#names5),*) => (#(#names6),*),
                        var => panic!("invalid variant {:?}", var),
                    }
                }
            }
        } else {
            let names1 = v.fields.iter().map(|f| f.ident.as_ref().unwrap());
            let names2 = names1.clone();
            let names3 = names1.clone();
            let names4 = names1.clone();
            let names5 = names1.clone();
            let names6 = names1.clone();
            
            let types1 = v.fields.iter().map(|f| &f.ty);
            let types2 = types1.clone();
            let types3 = types1.clone();
            

            quote::quote! {
                pub fn #getter_name(&self) -> (#( &#types1 ),*) {
                    match self {
                        Self::#ident { #(#names1),* } => (#(#names2),*),
                        var => panic!("invalid variant {:?}", var),
                    }
                }

                pub fn #mut_getter_name(&mut self) -> (#( &mut #types2 ),*) {
                    match self {
                        Self::#ident { #(#names4),* } => (#(#names3),*),
                        var => panic!("invalid variant {:?}", var),
                    }
                }

                pub fn #into_name(self) -> (#( #types3 ),*) {
                    match self {
                        Self::#ident { #(#names5),* } => (#(#names6),*),
                        var => panic!("invalid variant {:?}", var),
                    }
                }
            }
        }
    });

    quote::quote! {
        impl #generics #name #type_params {
            #( #functions )*
        }
    }.into()
}

#[proc_macro_derive(QuickDefault, attributes(default))]
pub fn derive_custom_default(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let name = &input.ident;

    let mut generics = input.generics.clone();
    for param in generics.type_params_mut() {
        param.bounds.push(parse_quote!(Default));
    }

    let mut type_params = input.generics.clone();
    for param in type_params.type_params_mut() {
        param.bounds.clear();
    }

    let data = match input.data {
        syn::Data::Struct(data) => data,
        _ => panic!("macro only supports structs"),
    };

    let fields = data.fields.iter().map(|f| {
        let attr = f.attrs.iter().find(|a| a.path.is_ident("default"));

        let ident = f.ident.as_ref().unwrap();

        if let Some(attr) = attr {
            let tokens = attr.tokens.clone();
            quote::quote!(
                #ident: #tokens
            )
        } else {
            quote::quote!(
                #ident: Default::default()
            )
        }
    });

    quote::quote!(
        impl #generics Default for #name #type_params {
            fn default() -> Self {
                Self {
                    #( #fields ),*
                }
            }
        }
    ).into()
}

fn pascal_to_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + s.chars().filter(|c| c.is_uppercase()).count());
    let mut prev_is_upper = true;
    for c in s.chars() {
        if c.is_uppercase() {
            if prev_is_upper {
                result.push(c.to_ascii_lowercase());
            } else {
                result.push('_');
                result.push(c.to_ascii_lowercase());
            }
            prev_is_upper = true;
        } else {
            result.push(c);
            prev_is_upper = false;
        }
    }
    result
}
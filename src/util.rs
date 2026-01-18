use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Field, Fields, LitStr};

type Body = proc_macro2::TokenStream;
type BodyIdent = proc_macro2::TokenStream;

// TODO:
// 1. There should be a From impl for Patch -> Field
// 2. Impl EnumIter for Fields -> this is to generate randomness for tests
// 3, If there is a flag #[test] at the top of the repo struct to impl a randomness generator

pub fn as_typed(ast: &DeriveInput,) -> (Body, BodyIdent,) {
    let fields = match &ast.data {
        Data::Struct(DataStruct { fields: Fields::Named(fields,), .. },) => &fields.named,
        _ => {
            return (
                syn::Error::new_spanned(&ast.ident, "expected a struct with named fields",)
                    .to_compile_error(),
                quote! { PatchField },
            );
        }
    };

    let mut to_arg = vec![];
    let mut to_string = vec![];
    let body_ident = quote! { PatchField };

    let typed_enum = fields.iter().map(|f| {
        let Some(name_ident,) = f.ident.as_ref() else {
            // Defensive: named fields should always have an ident.
            return syn::Error::new_spanned(f, "expected a named field (missing ident)",)
                .to_compile_error();
        };

        let ty = &f.ty;
        let name_str = name_ident.to_string();

        to_arg.push(quote! {
            #body_ident::#name_ident(arg) => args.add(arg)
        },);
        to_string.push(quote! {
            #body_ident::#name_ident(_) => #name_str.to_string()
        },);

        quote! { #name_ident(#ty) }
    },);

    let body = quote! {
        #[allow(non_snake_case, non_camel_case_types, nonstandard_style)]
        pub enum #body_ident {
            #(#typed_enum,)*
        }

        impl std::fmt::Display for #body_ident {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}", match self {
                    #(#to_string,)*
                })
            }
        }

        impl mae::repo::__private__::ToSqlParts for #body_ident {
            fn to_sql_parts(&self) -> mae::repo::__private__::AsSqlParts {
                // NOTE: cannot accurately get the bind_idx. Catch it at a higher level
                (vec![self.to_string()], None)

            }
        }

        impl mae::repo::__private__::BindArgs for #body_ident {
            fn bind(&self, mut args: &mut sqlx::postgres::PgArguments) {
                let _ = match self {
                    #(#to_arg,)*
                };
            }
            fn bind_len(&self) -> usize {
                // NOTE: There will always be one arg for a PatchField
                1
            }
        }
    };
    (body, body_ident,)
}

pub fn as_variant(ast: &DeriveInput,) -> (Body, BodyIdent,) {
    let fields = match &ast.data {
        Data::Struct(DataStruct { fields: Fields::Named(fields,), .. },) => &fields.named,
        _ => {
            return (
                syn::Error::new_spanned(&ast.ident, "expected a struct with named fields",)
                    .to_compile_error(),
                quote! { Field },
            );
        }
    };

    let mut all_cols: Vec<String,> = Vec::new();
    let mut to_string_arms: Vec<proc_macro2::TokenStream,> = Vec::new();
    let mut variants: Vec<proc_macro2::TokenStream,> = Vec::new();

    let body_ident = quote! { Field };

    for f in fields.iter() {
        let Some(name,) = f.ident.as_ref() else {
            variants.push(
                syn::Error::new_spanned(f, "expected a named field (missing ident)",)
                    .to_compile_error(),
            );
            continue;
        };

        let name_str = name.to_string();

        all_cols.push(name_str.clone(),);

        to_string_arms.push(quote! {
            #body_ident::#name => #name_str.to_string()
        },);

        variants.push(quote! { #name },);
    }

    let all_cols_str = all_cols.join(", ",);

    let body = quote! {
        #[allow(non_snake_case, non_camel_case_types, nonstandard_style)]
        pub enum #body_ident {
            All,
            #(#variants,)*
        }

        impl mae::repo::__private__::ToSqlParts for #body_ident {
            fn to_sql_parts(&self) -> mae::repo::__private__::AsSqlParts {
                (vec![self.to_string()], None)
            }
        }

        impl std::fmt::Display for #body_ident {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}", match self {
                    Self::All => #all_cols_str.into(),
                    #(#to_string_arms,)*
                })
            }
        }
    };

    (body, body_ident,)
}

pub fn as_option(ast: &DeriveInput,) -> (Body, BodyIdent,) {
    let fields = match &ast.data {
        Data::Struct(DataStruct { fields: Fields::Named(fields,), .. },) => &fields.named,
        _ => {
            return (
                syn::Error::new_spanned(&ast.ident, "expected a struct with named fields",)
                    .to_compile_error(),
                quote! { Row },
            );
        }
    };

    let body_ident = quote! { Row };

    let typed = fields.iter().map(|f| {
        let Some(name_ident,) = f.ident.as_ref() else {
            return syn::Error::new_spanned(f, "expected a named field (missing ident)",)
                .to_compile_error();
        };
        let ty = &f.ty;
        quote! { pub #name_ident: Option<#ty> }
    },);

    let string_some = fields.iter().map(|f| {
        let Some(name_ident,) = f.ident.as_ref() else {
            return syn::Error::new_spanned(f, "expected a named field (missing ident)",)
                .to_compile_error();
        };
        let name_str = name_ident.to_string();
        quote! {
            if let Some(v) = &self.#name_ident {
                sql.push(format!("{}", #name_str));
                sql_i.push(format!("${}", i));
                i += 1;
            }
        }
    },);

    let bind_some = fields.iter().map(|f| {
        let Some(name_ident,) = f.ident.as_ref() else {
            return syn::Error::new_spanned(f, "expected a named field (missing ident)",)
                .to_compile_error();
        };
        quote! {
            if let Some(v) = &self.#name_ident {
                let _ = args.add(v);
            }
        }
    },);

    let bind_len = fields.iter().map(|f| {
        let Some(name_ident,) = f.ident.as_ref() else {
            return syn::Error::new_spanned(f, "expected a named field (missing ident)",)
                .to_compile_error();
        };
        quote! {
            if let Some(v) = &self.#name_ident {
                count += 1;
            }
        }
    },);

    let body = quote! {
        #[allow(non_snake_case, non_camel_case_types, nonstandard_style)]
        pub struct #body_ident {
            #(#typed,)*
        }
        //
        // impl std::fmt::Display for #body_ident {
        //     fn fmt(&self, &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //         todo!()
        //     }
        // }

        impl #body_ident {
            fn sql(&self) -> mae::repo::__private__::AsSqlParts {
                let mut i = 1;
                let mut sql = vec![];
                let mut sql_i = vec![];
                #(#string_some)*

                (sql, Some(sql_i))
            }
        }

        impl mae::repo::__private__::ToSqlParts for #body_ident {
            fn to_sql_parts(&self) -> mae::repo::__private__::AsSqlParts {
                self.sql()

            }
        }

        impl mae::repo::__private__::BindArgs for #body_ident {
            fn bind(&self, mut args: &mut sqlx::postgres::PgArguments) {
                #(#bind_some)*
            }
            fn bind_len(&self) -> usize {
                let mut count = 0;
                #(#bind_len)*
                count
            }
        }
    };
    (body, body_ident,)
}

// Utils to find various attributes
fn find_get_attr(field: &Field, attr_name: &'static str,) -> Option<syn::Ident,> {
    let Some(ident,) = field.ident.clone() else {
        return None; // ignore tuple fields
    };

    for attr in &field.attrs {
        if attr.path().is_ident(attr_name,) {
            return Some(ident,);
        }
    }

    None
}
fn find_get_attr_with_args(
    field: &Field,
    attr_name: &'static str,
) -> Result<Option<(syn::Ident, String,),>, syn::Error,> {
    let Some(ident,) = field.ident.clone() else {
        return Ok(None,); // ignore tuple fields
    };

    for attr in &field.attrs {
        if attr.path().is_ident(attr_name,) {
            let lit: LitStr = attr.parse_args().map_err(|_| {
                syn::Error::new_spanned(attr, format!("expected #[{}(\"...\")]", attr_name),)
            },)?;
            return Ok(Some((ident, lit.value(),),),);
        }
    }

    Ok(None,)
}

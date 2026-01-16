#![deny(clippy::disallowed_methods)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types, nonstandard_style)]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data::Struct,
    DataStruct, DeriveInput, Fields,
    Fields::Named,
    FieldsNamed, Ident, ItemFn, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

mod util;
use util::*;

#[proc_macro_attribute]
pub fn run_app(_: TokenStream, input: TokenStream,) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    // Avoid indexing panic if the function body is empty.
    let fn_block = match input_fn.block.stmts.first() {
        Some(stmt,) => stmt,
        None => {
            return syn::Error::new_spanned(
                &input_fn.sig.ident,
                "run_app requires at least one statement in the function body",
            )
            .to_compile_error()
            .into();
        }
    };

    quote! {
    async fn run<Context: Clone + Send + 'static>(
        listener: TcpListener,
        db_pool: PgPool,
        base_url: String,
        hmac_secret: SecretString,
        redis_uri: SecretString,
        custom_context: Context,
    ) -> Result<Server, anyhow::Error> {

         let redis_store = app::redis_session(redis_uri).await?;
         let server = HttpServer::new(move || {
             ActixWebApp::new()
                 .wrap(TracingLogger::default())
                 .wrap(app::session_middleware(
                     hmac_secret.clone(),
                     redis_store.clone(),
                 ))
                 .app_data(web::Data::new(ApplicationBaseUrl(base_url.clone())))
                 .app_data(web::Data::new(HmacSecret(hmac_secret.clone())))
                 .app_data(web::Data::new(db_pool.clone()))
                 .app_data(web::Data::new(custom_context.clone()))
             .#fn_block
         })
         .listen(listener)?
         .run();
         Ok(server)
         }
         }
    .into()
}

struct Args {
    ctx: Ident,
    schema: LitStr,
    _comma: Token![,],
}

impl Parse for Args {
    fn parse(input: ParseStream<'_,>,) -> syn::Result<Self,> {
        Ok(Self { ctx: input.parse()?, _comma: input.parse()?, schema: input.parse()?, },)
    }
}

#[proc_macro_attribute]
pub fn schema(args: TokenStream, input: TokenStream,) -> TokenStream {
    let Args { ctx, schema, .. } = parse_macro_input!(args as Args);
    let ast = parse_macro_input!(input as DeriveInput);

    let repo_ident = &ast.ident;
    let repo_attrs = &ast.attrs;

    // confirm the macro is being called on a Struct Type and extract the fields.
    let fields = match ast.data {
        Struct(DataStruct { fields: Named(FieldsNamed { ref named, .. },), .. },) => named,
        _ => {
            return syn::Error::new_spanned(
                repo_ident,
                "schema only works for structs with named fields",
            )
            .to_compile_error()
            .into();
        }
    };

    // rebuild the struct fields
    let params = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        let attrs = &f.attrs;
        quote! {
            #(#attrs)*
            pub #name: #ty
        }
    },);

    // rebuild repo struct with the existing fields and default fields for the repo
    // NOTE: here, we are deriving the Repo with the proc_macro_derive fn from above
    let repo = quote! {

        #(#repo_attrs)*
        #[derive(mae_repo_macro::MaeRepo, Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize, Clone)]
        pub struct #repo_ident {
            #[id] pub id: i32,
            pub sys_client: i32,
            pub status: mae::repo::default::DomainStatus,
            #(#params,)*
            pub comment: Option<String>,
            #[sqlx(json)]
            pub tags: serde_json::Value,
            #[sqlx(json)]
            pub sys_detail: serde_json::Value,
            #[from_context] pub created_by: i32,
            #[from_context] pub updated_by: i32,
            #[gen_date] pub created_at: chrono::DateTime<chrono::Utc>,
            pub updated_at: chrono::DateTime<chrono::Utc>,
        }
        impl mae::repo::__private__::Build<#ctx, Row, Field, PatchField> for #repo_ident {
            fn schema() -> String {
                #schema.to_string()
            }
        }
    };
    repo.into()
}

// TODO:
//  attributes
//  1. from_context should take a function type to calculate it
//  2. gen_date should be changed to private_replace("now()") to replace the field's display +
//     BindArgs

#[proc_macro_derive(MaeRepo, attributes(id, from_context, gen_date))]
pub fn derive_mae_repo(item: TokenStream,) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    // Making sure it the derive macro is called on a struct;
    let _ = match &ast.data {
        Struct(DataStruct { fields: Fields::Named(fields,), .. },) => &fields.named,
        _ => {
            return syn::Error::new_spanned(
                &ast.ident,
                "MaeRepo derive expects a struct with named fields",
            )
            .to_compile_error()
            .into();
        }
    };

    let (repo_option, _,) = as_option(&ast,);
    let (repo_typed, _,) = as_typed(&ast,);
    let (repo_variant, _,) = as_variant(&ast,);

    quote! {
        #repo_option
        #repo_variant
        #repo_typed
    }
    .into()
}

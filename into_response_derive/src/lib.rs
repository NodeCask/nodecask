use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(TemplateResponse)]
pub fn derive_template_response(input: TokenStream) -> TokenStream {
    // 1. 解析输入的结构体代码
    let input = parse_macro_input!(input as DeriveInput);

    // 2. 获取结构体名称
    let name = input.ident;

    // 3. 获取泛型参数 (关键步骤)
    // split_for_impl 会把泛型拆分为：
    // impl_generics:  impl<T: Template>
    // ty_generics:    ModLayout<T>
    // where_clause:   where ...
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // 4. 生成代码
    let expanded = quote! {
        impl #impl_generics axum::response::IntoResponse for #name #ty_generics #where_clause {
            fn into_response(self) -> axum::response::Response {
                // 引入 askama::Template trait 才能调用 .render()
                use askama::Template;

                match self.render() {
                    Ok(html) => axum::response::Html(html).into_response(),
                    Err(err) => (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to render template. Error: {}", err),
                    )
                    .into_response(),
                }
            }
        }
        impl #impl_generics core::convert::Into<crate::common::WebResponse> for #name #ty_generics #where_clause {
            fn into(self)-> crate::common::WebResponse{
                crate::common::WebResponse::Ok(self.into_response())
            }
        }
    };

    // 5. 返回生成的代码
    TokenStream::from(expanded)
}
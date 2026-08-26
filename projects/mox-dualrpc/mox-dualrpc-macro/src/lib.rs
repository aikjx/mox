//! mox-dualrpc-macro: Zero-config dual-protocol RPC procedural macros
//!
//! Provides:
//! - `#[dual_rpc]` — Mark a method for dual-protocol exposure (standalone use)
//! - `#[dual_rpc_service]` — Auto-scan impl block, generate `register_routes()` method

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn, Meta, Type, punctuated::Punctuated, Token};

// === Shared helpers ===

/// Metadata extracted from a `#[dual_rpc(...)]` attribute
struct DualRpcMeta {
    jsonrpc_method: Option<String>,
    cache_ttl_ms: u64,
    cache_key: Option<String>,
    expose: bool,
    batch_supported: bool,
}

impl Default for DualRpcMeta {
    fn default() -> Self {
        Self {
            jsonrpc_method: None,
            cache_ttl_ms: 0,
            cache_key: None,
            expose: true,
            batch_supported: true,
        }
    }
}

/// Parse attribute arguments as a list of Meta items (syn 2.0 compatible)
fn parse_attr_args(attr: TokenStream) -> Vec<Meta> {
    use syn::parse::Parser;
    match Punctuated::<Meta, Token![,]>::parse_terminated.parse(attr) {
        Ok(parsed) => parsed.into_iter().collect(),
        Err(_) => Vec::new(),
    }
}

/// Extract DualRpcMeta from a list of attribute Meta items
fn extract_meta(metas: &[Meta]) -> DualRpcMeta {
    let mut result = DualRpcMeta::default();
    for meta in metas {
        if let Meta::NameValue(nv) = meta {
            let key = nv.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
            match key.as_str() {
                "method" => {
                    if let syn::Expr::Lit(lit) = &nv.value {
                        if let syn::Lit::Str(s) = &lit.lit {
                            result.jsonrpc_method = Some(s.value());
                        }
                    }
                }
                "cache_ttl_ms" => {
                    if let syn::Expr::Lit(lit) = &nv.value {
                        if let syn::Lit::Int(i) = &lit.lit {
                            result.cache_ttl_ms = i.base10_parse().unwrap_or(0);
                        }
                    }
                }
                "cache_key" => {
                    if let syn::Expr::Lit(lit) = &nv.value {
                        if let syn::Lit::Str(s) = &lit.lit {
                            result.cache_key = Some(s.value());
                        }
                    }
                }
                "expose" => {
                    if let syn::Expr::Lit(lit) = &nv.value {
                        if let syn::Lit::Bool(b) = &lit.lit {
                            result.expose = b.value;
                        }
                    }
                }
                "batch" => {
                    if let syn::Expr::Lit(lit) = &nv.value {
                        if let syn::Lit::Bool(b) = &lit.lit {
                            result.batch_supported = b.value;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    result
}

/// Find and extract `#[dual_rpc(...)]` metadata from a method's attributes.
/// Returns (metadata, was_present)
fn find_dual_rpc_attr(attrs: &[syn::Attribute]) -> (DualRpcMeta, bool) {
    for attr in attrs {
        if attr.path().is_ident("dual_rpc") {
            let metas: Vec<Meta> = match &attr.meta {
                Meta::List(list) => {
                    list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                        .map(|p| p.into_iter().collect())
                        .unwrap_or_default()
                }
                _ => Vec::new(),
            };
            return (extract_meta(&metas), true);
        }
    }
    (DualRpcMeta::default(), false)
}

/// Extract request type from method signature (second argument, after &self)
fn extract_request_type(sig: &syn::Signature) -> Option<Type> {
    let mut inputs = sig.inputs.iter();
    // Skip self
    let _self_arg = inputs.next()?;
    // Get request arg
    let req_arg = inputs.next()?;
    if let syn::FnArg::Typed(typed) = req_arg {
        Some((*typed.ty).clone())
    } else {
        None
    }
}

/// Extract response type from `Result<Response, Error>` return type
fn extract_response_type(sig: &syn::Signature) -> Option<Type> {
    if let syn::ReturnType::Type(_, ty) = &sig.output {
        if let Type::Path(type_path) = ty.as_ref() {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Result" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(ok_type)) = args.args.first() {
                            return Some(ok_type.clone());
                        }
                    }
                }
            }
        }
    }
    None
}

// === #[dual_rpc] standalone attribute ===

/// `#[dual_rpc]` — Mark a method for dual-protocol (gRPC + JSON-RPC) exposure.
///
/// When used standalone (outside `#[dual_rpc_service]`), generates a metadata
/// helper function. When used inside `#[dual_rpc_service]`, it acts as a marker
/// and is stripped by the service macro.
///
/// # Arguments
/// - `method = "..."` — JSON-RPC method name (auto-generated if omitted)
/// - `cache_ttl_ms = N` — Response cache TTL in milliseconds (0 = no cache)
/// - `cache_key = "..."` — Cache key template
/// - `expose = true/false` — Whether to expose via JSON-RPC (default true)
/// - `batch = true/false` — Whether to support batch requests (default true)
#[proc_macro_attribute]
pub fn dual_rpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_meta = parse_attr_args(attr);
    let meta = extract_meta(&attr_meta);
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    let jsonrpc_method = meta.jsonrpc_method.unwrap_or_else(|| format!("method.{}", fn_name_str));
    let cache_ttl_ms = meta.cache_ttl_ms;
    let expose = meta.expose;
    let batch_supported = meta.batch_supported;
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let block = &input_fn.block;
    let inputs = &sig.inputs;
    let output = &sig.output;
    let asyncness = &sig.asyncness;

    let register_fn = format_ident!("__register_dualrpc_{}", fn_name_str);

    let expanded = quote! {
        #vis #asyncness fn #fn_name(#inputs) #output #block

        #[doc(hidden)]
        #[allow(non_snake_case)]
        pub fn #register_fn() -> (&'static str, &'static str, u64, bool, bool) {
            (#jsonrpc_method, #fn_name_str, #cache_ttl_ms, #expose, #batch_supported)
        }
    };

    let _ = meta.cache_key;
    TokenStream::from(expanded)
}

// === #[dual_rpc_service] auto-registration attribute ===

/// `#[dual_rpc_service]` — Auto-scan an impl block for `#[dual_rpc]` methods
/// and generate a `register_routes()` method.
///
/// # Example
///
/// ```ignore
/// #[dual_rpc_service]
/// impl MyService {
///     #[dual_rpc(method = "my.service.DoThing", cache_ttl_ms = 1000)]
///     async fn do_thing(&self, req: DoThingRequest) -> Result<DoThingResponse, DualRpcError> {
///         // ...
///     }
/// }
///
/// // Generated automatically:
/// // impl MyService {
/// //     pub fn register_routes(&self) -> Vec<mox_dualrpc::registry::RouteEntry> { ... }
/// // }
/// ```
///
/// # Requirements
/// - The service type must implement `Clone`
/// - Each `#[dual_rpc]` method must take `&self` and one request parameter
/// - Each `#[dual_rpc]` method must return `Result<Response, Error>`
/// - Request and Response types must implement `serde::Serialize + serde::de::DeserializeOwned`
#[proc_macro_attribute]
pub fn dual_rpc_service(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_impl = parse_macro_input!(item as syn::ItemImpl);

    // Collect route info from methods
    let mut route_infos: Vec<RouteInfo> = Vec::new();

    // Process items in impl block
    for item in &mut input_impl.items {
        if let syn::ImplItem::Fn(method) = item {
            let (meta, present) = find_dual_rpc_attr(&method.attrs);
            if present {
                // Strip the #[dual_rpc] attribute
                method.attrs.retain(|a| !a.path().is_ident("dual_rpc"));

                let fn_name = method.sig.ident.clone();
                let fn_name_str = fn_name.to_string();
                let jsonrpc_method = meta.jsonrpc_method
                    .unwrap_or_else(|| format!("method.{}", fn_name_str));
                let request_type = extract_request_type(&method.sig);
                let response_type = extract_response_type(&method.sig);

                route_infos.push(RouteInfo {
                    fn_name,
                    fn_name_str,
                    jsonrpc_method,
                    cache_ttl_ms: meta.cache_ttl_ms,
                    cache_key: meta.cache_key,
                    expose: meta.expose,
                    batch_supported: meta.batch_supported,
                    request_type,
                    response_type,
                });
            }
        }
    }

    // Generate register_routes method
    let route_entries = route_infos.iter().map(|info| {
        let fn_name = &info.fn_name;
        let jsonrpc_method = &info.jsonrpc_method;
        let fn_name_str = &info.fn_name_str;
        let cache_ttl_ms = info.cache_ttl_ms;
        let expose = info.expose;
        let batch_supported = info.batch_supported;

        let cache_key_tokens = info.cache_key.as_ref()
            .map(|k| quote! { Some(#k) })
            .unwrap_or(quote! { None });

        let request_type = info.request_type.as_ref()
            .map(|t| quote! { #t })
            .unwrap_or(quote! { serde_json::Value });

        let response_type = info.response_type.as_ref()
            .map(|t| quote! { #t })
            .unwrap_or(quote! { serde_json::Value });

        quote! {
            {
                let __svc = self.clone();
                mox_dualrpc::registry::make_route(
                    mox_dualrpc::registry::RouteMeta {
                        jsonrpc_method: #jsonrpc_method,
                        grpc_method: #fn_name_str,
                        cache_ttl_ms: #cache_ttl_ms,
                        cache_key: #cache_key_tokens,
                        expose: #expose,
                        batch_supported: #batch_supported,
                    },
                    move |__params: serde_json::Value| {
                        let __svc = __svc.clone();
                        async move {
                            let __req: #request_type = serde_json::from_value(__params)
                                .map_err(|e| mox_dualrpc::error::DualRpcError::Serialization(e))?;
                            let __resp: #response_type = __svc.#fn_name(__req).await
                                .map_err(|e| mox_dualrpc::error::DualRpcError::Other(e.to_string()))?;
                            let __json = serde_json::to_value(__resp)
                                .map_err(|e| mox_dualrpc::error::DualRpcError::Serialization(e))?;
                            Ok(__json)
                        }
                    },
                )
            }
        }
    });

    let route_count = route_infos.len();

    let register_method = quote! {
        /// Auto-generated by #[dual_rpc_service] — register all #[dual_rpc] methods as routes
        pub fn register_routes(&self) -> Vec<mox_dualrpc::registry::RouteEntry> {
            let mut __routes = Vec::with_capacity(#route_count);
            #( __routes.push(#route_entries); )*
            __routes
        }
    };

    // Add register_routes to impl items
    input_impl.items.push(syn::ImplItem::Verbatim(register_method));

    let expanded = quote! { #input_impl };
    TokenStream::from(expanded)
}

// === Internal types for route info collection ===

struct RouteInfo {
    fn_name: syn::Ident,
    fn_name_str: String,
    jsonrpc_method: String,
    cache_ttl_ms: u64,
    cache_key: Option<String>,
    expose: bool,
    batch_supported: bool,
    request_type: Option<Type>,
    response_type: Option<Type>,
}

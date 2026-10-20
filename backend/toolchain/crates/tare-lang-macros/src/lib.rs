use heck::{CamelCase, SnakeCase};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::{format_ident, quote, ToTokens};
use sha2::{Digest, Sha256};
use syn::{
    parse_macro_input, spanned::Spanned, DeriveInput, Expr, FnArg, GenericArgument, Ident,
    ItemEnum, ItemMod, ItemStruct, PathArguments, Token, Type,
};
const TARE_ANCHOR_MAJOR: u8 = 0;
#[path="declare_program.rs"]
mod declare_program_impl;

fn discriminator(namespace: &str, name: &str) -> Tokens {
    let hash = Sha256::digest(format!("{}:{}", namespace, name));
    let bytes = &hash[..8];
    quote! { &[#(#bytes),*] }
}
fn fail<T: quote::ToTokens>(value: T, construct: impl std::fmt::Display) -> Tokens {
    syn::Error::new_spanned(value, format!("Tare does not implement {}", construct))
        .to_compile_error()
}
#[proc_macro_attribute]
pub fn error_code(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as ItemEnum);
    let name = &item.ident;
    let offset = if args.is_empty() {
        6000u32
    } else {
        let parsed = syn::parse::<syn::MetaNameValue>(args);
        match parsed {
            Ok(meta) if meta.path.is_ident("offset") => match meta.lit {
                syn::Lit::Int(n) => match n.base10_parse::<u32>() {
                    Ok(n) => n,
                    Err(e) => return e.to_compile_error().into(),
                },
                v => return fail(v, "error_code offset").into(),
            },
            Ok(meta) => return fail(meta, "error_code argument").into(),
            Err(e) => return e.to_compile_error().into(),
        }
    };
    let mut variants = Vec::new();
    let mut arms = Vec::new();
    for variant in &mut item.variants {
        let id = &variant.ident;
        let text = id.to_string();
        let mut message = text.clone();
        for attr in &variant.attrs {
            if attr.path.is_ident("msg") {
                match attr.parse_args::<syn::LitStr>() {
                    Ok(s) => message = s.value(),
                    Err(e) => return e.to_compile_error().into(),
                }
            }
        }
        variant.attrs.retain(|a| !a.path.is_ident("msg"));
        let code = quote! {#offset + #name::#id as u32};
        arms.push(quote! { #name::#id => anchor_lang::error::Error::from_static(&anchor_lang::error::ErrorDescriptor {code:#code,name:#text,message:#message}) });
        variants.push(quote! { #name::#id => #code });
    }
    quote! { #[derive(Debug, Clone, Copy)] #item impl From<#name> for anchor_lang::error::Error { #[inline(always)] fn from(value: #name) -> Self { match value { #(#arms),* } } } impl From<#name> for u32 { fn from(value: #name) -> u32 { match value { #(#variants),* } } } }.into()
}
#[proc_macro_attribute]
pub fn account(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    let (zero, namespace, override_disc) = match account_options(args) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    let name = &item.ident;
    let text = name.to_string();
    let custom_disc = override_disc.is_some();
    let disc = override_disc
        .unwrap_or_else(|| discriminator(namespace.as_deref().unwrap_or("account"), &text));
    if let Some(is_unsafe) = zero {
        let attr = if is_unsafe {
            quote! {#[anchor_lang::zero_copy(unsafe)]}
        } else {
            quote! {#[anchor_lang::zero_copy]}
        };
        let (ig, tg, wg) = item.generics.split_for_impl();
        let pod = if is_unsafe {
            quote! {unsafe impl #ig anchor_lang::__private::bytemuck::Pod for #name #tg #wg {} unsafe impl #ig anchor_lang::__private::bytemuck::Zeroable for #name #tg #wg {}}
        } else {
            quote! {}
        };
        let owner = if namespace.is_none() {
            quote! {impl #ig anchor_lang::Owner for #name #tg #wg {fn owner()->anchor_lang::Pubkey{crate::ID}}}
        } else {
            quote! {}
        };
        return quote!{
            #attr #item #pod #owner
            impl #ig anchor_lang::ZeroCopy for #name #tg #wg {}
            impl #ig anchor_lang::Discriminator for #name #tg #wg {const DISCRIMINATOR:&'static[u8]=#disc;}
            impl #ig anchor_lang::AccountDeserialize for #name #tg #wg {
                fn discriminator()->&'static[u8]{<Self as anchor_lang::Discriminator>::DISCRIMINATOR}
                fn try_deserialize(buf:&mut &[u8])->anchor_lang::Result<Self>{
                    let disc=<Self as anchor_lang::Discriminator>::DISCRIMINATOR;
                    if buf.len()<disc.len(){return Err(anchor_lang::ErrorCode::AccountDiscriminatorNotFound.into());}
                    if &buf[..disc.len()]!=disc{return Err(anchor_lang::error!(anchor_lang::ErrorCode::AccountDiscriminatorMismatch).__tare_with_account_name_static(#text));}
                    Self::try_deserialize_unchecked(buf)
                }
                fn try_deserialize_unchecked(buf:&mut &[u8])->anchor_lang::Result<Self>{Ok(*anchor_lang::__private::bytemuck::from_bytes(&buf[<Self as anchor_lang::Discriminator>::DISCRIMINATOR.len()..]))}
            }
        }.into();
    }
    let (impl_g, ty_g, where_g) = item.generics.split_for_impl();
    let mut offset = 0usize;
    let mut offsets = Vec::new();
    let mut direct_types = Vec::new();
    let mut direct_possible = !custom_disc
        && namespace.is_none()
        && !item.attrs.iter().any(|a| a.path.is_ident("borsh_init") || a.path.is_ident("borsh"));
    let mut fast_reads = Vec::new();
    let mut fast_writes = Vec::new();
    for field in &item.fields {
        if field.attrs.iter().any(|a| !a.path.is_ident("doc")) {
            direct_possible = false;
        }
        let width = if let Type::Path(p) = &field.ty {
            match p.path.segments.last().unwrap().ident.to_string().as_str() {
                "u8" | "i8" => Some(1),
                "u16" | "i16" => Some(2),
                "u32" | "i32" => Some(4),
                "u64" | "i64" => Some(8),
                "u128" | "i128" => Some(16),
                "Pubkey" => Some(32),
                _ => None,
            }
        } else {
            None
        };
        if let (Some(width), Some(id)) = (width, &field.ident) {
            let ty = &field.ty;
            let Type::Path(p) = ty else { unreachable!() };
            let primitive = &p.path.segments.last().unwrap().ident;
            let canonical = if primitive == "Pubkey" {
                quote! {anchor_lang::Pubkey}
            } else {
                quote! {::core::primitive::#primitive}
            };
            direct_types.push(
                quote! {&& std::any::type_name::<#ty>()==std::any::type_name::<#canonical>()},
            );
            offsets.push(quote! {&& std::mem::offset_of!(Self,#id)==#offset});
            fast_reads.push(quote!{#id:unsafe {std::ptr::read_unaligned(data.as_ptr().add(#offset).cast::<#ty>())}});
            let wire_offset = 8 + offset;
            let word = match width {
                1 => quote! {::core::primitive::u8},
                2 => quote! {::core::primitive::u16},
                4 => quote! {::core::primitive::u32},
                _ => quote! {::core::primitive::u64},
            };
            let chunk = width.min(8);
            for offset in (0..width).step_by(chunk) {
                let target = wire_offset + offset;
                fast_writes.push(quote!{unsafe {
                    let value=std::ptr::read_unaligned(std::ptr::addr_of!(self.#id).cast::<u8>().add(#offset).cast::<#word>());
                    let target=data.as_mut_ptr().add(#target).cast::<#word>();
                    if std::ptr::read_unaligned(target)!=value {std::ptr::write_unaligned(target,value);}
                }});
            }
            offset += width;
        } else {
            direct_possible = false;
        }
    }
    let body_len = offset;
    let fast_deserialize = if direct_possible {
        quote! {
            if cfg!(target_endian="little") && (true #(#direct_types)*) {
                if data.len()<#body_len {return Err(anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into());}
                return Ok(Self {#(#fast_reads),*});
            }
        }
    } else {
        quote! {}
    };
    let fast_serialize = if direct_possible {
        quote! {
            #[inline(always)] fn try_serialize_slice(&self,data:&mut[u8])->anchor_lang::Result<()> {
                if cfg!(target_endian="little") && (true #(#direct_types)*) && !std::mem::needs_drop::<Self>() {
                    if data.len()<8+#body_len {return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());}
                    data[..8].copy_from_slice(<Self as anchor_lang::Discriminator>::DISCRIMINATOR);
                    #(#fast_writes)*
                    return Ok(());
                }
                self.try_serialize(&mut &mut *data)
            }
        }
    } else {
        quote! {}
    };
    // Fixed primitive fields can use their wire representation even when a
    // different field requires the ordinary Borsh implementation. Keep each
    // field in declaration order so partial writes and failures agree.
    fn canonical_wire(ty:&Type)->Option<Tokens>{
        match ty {
            Type::Path(path) if path.qself.is_none()=>{
                let id=&path.path.segments.last()?.ident;
                match id.to_string().as_str(){
                    "u8"|"i8"|"u16"|"i16"|"u32"|"i32"|"u64"|"i64"|"u128"|"i128"=>Some(quote!(::core::primitive::#id)),
                    "Pubkey"=>Some(quote!(anchor_lang::Pubkey)),
                    _=>None,
                }
            },
            Type::Array(array)=>{let inner=canonical_wire(&array.elem)?;let len=&array.len;Some(quote!([#inner;#len]))},
            _=>None,
        }
    }
    let mixed = !direct_possible && item.generics.params.is_empty()
        && !item.attrs.iter().any(|a|a.path.is_ident("borsh_init") || a.path.is_ident("borsh"))
        && item.fields.iter().all(|f|f.ident.is_some() && f.attrs.iter().all(|a|a.path.is_ident("doc")));
    let mut mixed_read=Vec::new();let mut mixed_write=Vec::new();let mut mixed_names=Vec::new();
    if mixed {
        for field in &item.fields {
            let id=field.ident.as_ref().unwrap();let ty=&field.ty;mixed_names.push(id);
            let fallback_read=quote!(<#ty as anchor_lang::AnchorDeserialize>::deserialize(&mut __tare_account_bytes).map_err(|_|anchor_lang::ErrorCode::AccountDidNotDeserialize)?);
            let fallback_write=quote!(anchor_lang::AnchorSerialize::serialize(&self.#id,&mut __tare_account_bytes).map_err(|_|anchor_lang::ErrorCode::AccountDidNotSerialize)?;);
            if let Some(canonical)=canonical_wire(ty) {
                mixed_read.push(quote!{let #id:#ty=if cfg!(target_endian="little") && std::any::type_name::<#ty>()==std::any::type_name::<#canonical>() {
                    let n=std::mem::size_of::<#canonical>();if __tare_account_bytes.len()<n{return Err(anchor_lang::ErrorCode::AccountDidNotDeserialize.into());}
                    let value=unsafe{std::ptr::read_unaligned(__tare_account_bytes.as_ptr().cast::<#ty>())};__tare_account_bytes=&__tare_account_bytes[n..];value
                }else{#fallback_read};});
                mixed_write.push(quote!{if cfg!(target_endian="little") && std::any::type_name::<#ty>()==std::any::type_name::<#canonical>() {
                    let n=std::mem::size_of::<#canonical>();let bytes=unsafe{std::slice::from_raw_parts(std::ptr::addr_of!(self.#id).cast::<u8>(),n)};
                    std::io::Write::write_all(&mut __tare_account_bytes,bytes).map_err(|_|anchor_lang::ErrorCode::AccountDidNotSerialize)?;
                }else{#fallback_write}});
            }else{
                mixed_read.push(quote!{let #id:#ty=#fallback_read;});mixed_write.push(fallback_write);
            }
        }
    }
    let deserialize_body=if mixed {quote!{let mut __tare_account_bytes=data; #(#mixed_read)* Ok(Self{#(#mixed_names),*})}}else{quote!{
        #fast_deserialize
        anchor_lang::AnchorDeserialize::deserialize(&mut data).map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
    }};
    let fast_serialize=if mixed {quote!{
        #[inline(always)]fn try_serialize_slice(&self,mut __tare_account_bytes:&mut[u8])->anchor_lang::Result<()>{
            std::io::Write::write_all(&mut __tare_account_bytes,<Self as anchor_lang::Discriminator>::DISCRIMINATOR).map_err(|_|anchor_lang::ErrorCode::AccountDidNotSerialize)?;
            #(#mixed_write)* Ok(())
        }
    }}else{fast_serialize};
    let direct_impl = if direct_possible {
        quote! {
            #[inline(always)] unsafe fn try_direct(info:&anchor_lang::AccountView, unchecked:bool)->anchor_lang::Result<Option<*mut Self>> {
                if !(true #(#offsets)* #(#direct_types)*) || std::mem::needs_drop::<Self>() {return Ok(None);}
                if !info.__is_raw(){return Ok(None);}
                let data=info.__try_borrow_raw_data()?;
                if !unchecked {
                    if data.len()<8 {return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());}
                    if &data[..8]!=<Self as anchor_lang::Discriminator>::DISCRIMINATOR {return Err(anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch).__tare_with_account_name_static(#text));}
                }
                if unchecked {let _=&data[8..];}
                if data.len()<8+#body_len {return Err(anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into());}
                let pointer=info.__data_ptr().add(8).cast::<Self>();
                // The loader aligns account data to eight bytes. Larger
                // explicit Rust alignment still needs a dynamic check.
                if std::mem::align_of::<Self>() > 8 && pointer as usize % std::mem::align_of::<Self>() != 0 {return Ok(None);}
                Ok(Some(pointer))
            }
        }
    } else {
        quote! {}
    };
    let owner_impl = if namespace.as_deref().unwrap_or("").is_empty() {
        quote! {impl #impl_g anchor_lang::Owner for #name #ty_g #where_g {fn owner()->anchor_lang::Pubkey{crate::ID}}}
    } else {
        quote! {}
    };
    quote! {
        #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone)] #item
        impl #impl_g anchor_lang::Discriminator for #name #ty_g #where_g { const DISCRIMINATOR: &'static [u8] = #disc; }
        #owner_impl
        impl #impl_g anchor_lang::AccountSerialize for #name #ty_g #where_g {
            #fast_serialize
            fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
                writer.write_all(<Self as anchor_lang::Discriminator>::DISCRIMINATOR).map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotSerialize)?;
                anchor_lang::AnchorSerialize::serialize(self, writer).map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotSerialize.into())
            }
        }
        impl #impl_g anchor_lang::AccountDeserialize for #name #ty_g #where_g {
            #direct_impl
            const __TARE_CANONICAL_DIRECT:bool=#direct_possible;
            fn discriminator()->&'static [u8] {<Self as anchor_lang::Discriminator>::DISCRIMINATOR}
            #[inline(always)] fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
                let disc = <Self as anchor_lang::Discriminator>::DISCRIMINATOR;
                if buf.len() < disc.len() { return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into()); }
                if &buf[..disc.len()] != disc { return Err(anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch).__tare_with_account_name_static(#text)); }
                Self::try_deserialize_unchecked(buf)
            }
            #[inline(always)] fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
                let disc = <Self as anchor_lang::Discriminator>::DISCRIMINATOR;
                let mut data = &buf[disc.len()..];
                #deserialize_body
            }
        }
    }.into()
}
// Direct account views are confined to handlers whose effects can only be field writes.
// Unknown calls, raw account access, loops, closures and macros retain Anchor snapshots.
fn context_account(expr: &Expr, ctx: &Ident) -> bool {
    if let Expr::Field(account) = expr {
        if let Expr::Field(accounts) = &*account.base {
            if accounts.member == syn::Member::Named(format_ident!("accounts")) {
                if let Expr::Path(root) = &*accounts.base {
                    return root.path.is_ident(ctx);
                }
            }
        }
    }
    false
}
fn plain_block(block: &syn::Block, ctx: &Ident) -> bool {
    block.stmts.iter().all(|s| match s {
        syn::Stmt::Local(local) => local
            .init
            .as_ref()
            .map(|(_, expr)| plain_expr(expr, ctx))
            .unwrap_or(true),
        syn::Stmt::Expr(e) | syn::Stmt::Semi(e, _) => plain_expr(e, ctx),
        _ => false,
    })
}
fn plain_expr(e: &Expr, ctx: &Ident) -> bool {
    match e {
        Expr::Lit(_) | Expr::Path(_) => true,
        Expr::Field(x) => plain_expr(&x.base, ctx),
        Expr::Reference(x) => plain_expr(&x.expr, ctx),
        Expr::Unary(x) => plain_expr(&x.expr, ctx),
        Expr::Paren(x) => plain_expr(&x.expr, ctx),
        Expr::Group(x) => plain_expr(&x.expr, ctx),
        Expr::Binary(x) => plain_expr(&x.left, ctx) && plain_expr(&x.right, ctx),
        Expr::Assign(x) => plain_expr(&x.left, ctx) && plain_expr(&x.right, ctx),
        Expr::AssignOp(x) => plain_expr(&x.left, ctx) && plain_expr(&x.right, ctx),
        Expr::Tuple(x) => x.elems.iter().all(|x| plain_expr(x, ctx)),
        Expr::Call(x) => {
            matches!(&*x.func,Expr::Path(p) if p.path.is_ident("Ok"))
                && x.args.iter().all(|x| plain_expr(x, ctx))
        }
        Expr::MethodCall(x) => {
            (x.method == "key" || x.method == "deref_mut")
                && context_account(&x.receiver, ctx)
                && x.args.is_empty()
        }
        Expr::Return(x) => x.expr.as_ref().map(|x| plain_expr(x, ctx)).unwrap_or(true),
        Expr::Block(x) => plain_block(&x.block, ctx),
        _ => false,
    }
}
#[proc_macro_attribute]
pub fn program(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemMod);
    if !args.is_empty() {
        return fail(args.to_string(), "program attribute arguments").into();
    }
    let modname = &item.ident;
    let Some((_, items)) = &item.content else {
        return fail(&item, "out-of-line program modules").into();
    };
    let instruction_count = items
        .iter()
        .filter(|i| matches!(i, syn::Item::Fn(_)))
        .count();
    let mut inline_handlers=Vec::new();
    let mut dispatch = Vec::new();
    let mut wrappers = Vec::new();
    let mut instructions = Vec::new();
    let mut cpi_methods = Vec::new();
    let mut cpi_exports = std::collections::BTreeMap::new();
    let mut fallback = None;
    for part in items {
        let syn::Item::Fn(fun) = part else { continue };
        let return_ty: Type = match &fun.sig.output {
            syn::ReturnType::Type(_, ty) => match &**ty {
                Type::Path(p) => match &p.path.segments.last().unwrap().arguments {
                    PathArguments::AngleBracketed(a) => match a.args.first() {
                        Some(GenericArgument::Type(t)) => t.clone(),
                        _ => return fail(ty, "instruction Result return type").into(),
                    },
                    _ => return fail(ty, "instruction Result return type").into(),
                },
                _ => return fail(ty, "instruction Result return type").into(),
            },
            _ => return fail(&fun.sig, "instruction Result return type").into(),
        };
        let unit_return = matches!(&return_ty,Type::Tuple(t) if t.elems.is_empty());
        if !unit_return {inline_handlers.push(fun.sig.ident.clone());}
        let name = &fun.sig.ident;
        let cfgs: Vec<_> = fun.attrs.iter().filter(|attr|attr.path.is_ident("cfg")).collect();
        let camel = format_ident!("{}", name.to_string().to_camel_case());
        let wrapper = format_ident!("__tare_{}", name);
        let Some(FnArg::Typed(first)) = fun.sig.inputs.first() else {
            return fail(fun, "instruction without Context").into();
        };
        let context = match &*first.ty {
            Type::Path(context) if context.path.segments.last().is_some_and(|p|p.ident=="Context") => context,
            _ => {
                if fallback.is_some() { return fail(fun,"multiple fallback functions").into(); }
                fallback=Some(name.clone());
                continue;
            }
        };
        let segment = context.path.segments.last().unwrap();
        let PathArguments::AngleBracketed(generics) = &segment.arguments else {
            return fail(first, "Context without account type").into();
        };
        let Some(GenericArgument::Type(accounts_ty)) = generics.args.last() else {
            return fail(first, "Context account type").into();
        };
        let needs_remaining=match &*first.pat {
            syn::Pat::Ident(pat)=>context_may_read_remaining(fun,&pat.ident),
            _=>true,
        };
        let primitive_args = fun.sig.inputs.iter().skip(1).all(|arg| match arg {
            FnArg::Typed(a) => match &*a.ty {
                Type::Path(p) => {
                    p.qself.is_none()
                        && p.path.segments.len() == 1
                        && matches!(
                            p.path.segments.last().unwrap().ident.to_string().as_str(),
                            "bool"
                                | "u8"
                                | "i8"
                                | "u16"
                                | "i16"
                                | "u32"
                                | "i32"
                                | "u64"
                                | "i64"
                                | "u128"
                                | "i128"
                                | "f32"
                                | "f64"
                                | "Pubkey"
                        )
                }
                _ => false,
            },
            _ => false,
        });
        let direct = primitive_args && !needs_remaining
            && if let syn::Pat::Ident(pat) = &*first.pat {
                plain_block(&fun.block, &pat.ident)
            } else {
                false
            };
        let mut read_args = Vec::new();
        let mut call_args = Vec::new();
        let mut fields = Vec::new();
        let mut direct_args = Vec::new();
        let mut cpi_args = Vec::new();
        let override_discriminator=match instruction_discriminator(fun) {Ok(value)=>value,Err(error)=>return error.to_compile_error().into()};
        let mut wire_size = if override_discriminator.is_some(){None}else{Some(8usize)};
        for arg in fun.sig.inputs.iter().skip(1) {
            let FnArg::Typed(arg) = arg else {
                return fail(fun, "self instruction argument").into();
            };
            let syn::Pat::Ident(pat) = &*arg.pat else {
                return fail(arg, "destructured argument").into();
            };
            let arg_name = &pat.ident;
            let ty = &arg.ty;
            cpi_args.push(quote! {#arg_name:#ty});
            let width = if let Type::Path(p) = &**ty {
                match p.path.segments.last().unwrap().ident.to_string().as_str() {
                    "bool" | "u8" | "i8" => Some(1),
                    "u16" | "i16" => Some(2),
                    "u32" | "i32" | "f32" => Some(4),
                    "u64" | "i64" | "f64" => Some(8),
                    "u128" | "i128" => Some(16),
                    "Pubkey" => Some(32),
                    _ => None,
                }
            } else {
                None
            };
            wire_size = wire_size.and_then(|size| width.map(|width| size + width));
            if primitive_args {
                if let Type::Path(p) = &**ty {
                    let primitive = &p.path.segments.last().unwrap().ident;
                    let canonical = if primitive == "Pubkey" {
                        quote! {anchor_lang::Pubkey}
                    } else {
                        quote! {::core::primitive::#primitive}
                    };
                    direct_args.push(quote!{&& std::any::type_name::<#ty>()==std::any::type_name::<#canonical>()});
                }
            }
            fields.push(quote! {pub #arg_name:#ty});
            let primitive_decode=if let (Some(width),Type::Path(path))=(width,&**ty) {
                let name=path.path.segments.last().unwrap().ident.to_string();
                if path.qself.is_none() && path.path.segments.len()==1 && matches!(name.as_str(),"u8"|"i8"|"u16"|"i16"|"u32"|"i32"|"u64"|"i64"|"u128"|"i128"|"bool"|"Pubkey") {
                    let primitive=&path.path.segments.last().unwrap().ident;
                    let canonical=if name=="Pubkey"{quote!{anchor_lang::Pubkey}}else{quote!{::core::primitive::#primitive}};
                    let validity=if name=="bool"{quote!{if __data[0]>1{return Err(anchor_lang::error::ErrorCode::InstructionDidNotDeserialize.into());}}}else{quote!{}};
                    Some(quote! {
                        if cfg!(target_endian="little") && std::any::type_name::<#ty>()==std::any::type_name::<#canonical>() {
                            if __data.len()<#width{return Err(anchor_lang::error::ErrorCode::InstructionDidNotDeserialize.into());}
                            #validity
                            let value=unsafe{std::ptr::read_unaligned(__data.as_ptr().cast::<#ty>())};
                            __data=&__data[#width..];value
                        }else{anchor_lang::AnchorDeserialize::deserialize(&mut __data).map_err(|_|anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?}
                    })
                } else {None}
            }else{None};
            if let Some(decode)=primitive_decode {read_args.push(quote!{let #arg_name:#ty={#decode};});}
            else {read_args.push(quote!{let #arg_name: #ty = anchor_lang::AnchorDeserialize::deserialize(&mut __data).map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;});}
            call_args.push(arg_name);
        }
        let wrapper_inline = if instruction_count>16 {
            quote! {#[cfg_attr(tare_sdk, inline(never))] #[cfg_attr(not(tare_sdk), inline)]}
        } else if direct && instruction_count <= 2 {
            quote! {#[inline(always)]}
        } else {
            quote! {#[inline]}
        };
        let maybe_return = if unit_return {
            quote! {}
        } else {
            quote! {
                let mut __return_data=Vec::with_capacity(256);
                anchor_lang::AnchorSerialize::serialize(&__result,&mut __return_data).unwrap();
                anchor_lang::solana_program::program::set_return_data(&__return_data);
            }
        };
        let log = format!("Instruction: {}", camel);
        let disc = override_discriminator.unwrap_or_else(||discriminator("global", &name.to_string()));
        dispatch.push(quote!{#(#cfgs)* if data.starts_with(#disc) { return #wrapper::<RAW>(program_id, &mut input.cursor_mode::<RAW>(), &data[<instruction::#camel as anchor_lang::Discriminator>::DISCRIMINATOR.len()..]); }});
        let (remaining_setup,remaining_slice)=if needs_remaining {
            (quote!{let __remaining_storage=__cursor.remaining_storage();},quote!{__remaining_storage.as_slice()})
        }else{(quote!{},quote!{&[]})};
        let slot_body=if needs_remaining {quote!{
                    let mut __accounts=unsafe {__accounts_storage.assume_init()};
                    #remaining_setup
                    let __result=#modname::#name(anchor_lang::Context::new(__program_id, &mut __accounts, #remaining_slice, __bumps), #(#call_args),*)?;
                    #maybe_return
                    anchor_lang::Accounts::exit(&__accounts, __program_id)
                    }}else{quote!{                    #[cfg(tare_sdk)]
                    {
                        struct __TareSlot<'a,T>(&'a mut std::mem::MaybeUninit<T>);
                        impl<T> Drop for __TareSlot<'_,T>{#[inline(always)]fn drop(&mut self){unsafe{self.0.assume_init_drop();}}}
                        #remaining_setup
                        let mut __slot=__TareSlot(&mut __accounts_storage);
                        let __accounts=unsafe {__slot.0.assume_init_mut()};
                        let __result=#modname::#name(anchor_lang::Context::new(__program_id, &mut *__accounts, #remaining_slice, __bumps), #(#call_args),*)?;
                        #maybe_return
                        anchor_lang::Accounts::exit(&*__accounts, __program_id)
                    }
                    #[cfg(not(tare_sdk))]
                    {
                    let mut __accounts=unsafe {__accounts_storage.assume_init()};
                    #remaining_setup
                    let __result=#modname::#name(anchor_lang::Context::new(__program_id, &mut __accounts, #remaining_slice, __bumps), #(#call_args),*)?;
                    #maybe_return
                    anchor_lang::Accounts::exit(&__accounts, __program_id)
                    }}};
        wrappers.push(quote!{
            #(#cfgs)* #wrapper_inline fn #wrapper<'info,const RAW:bool>(__program_id: &anchor_lang::Pubkey, __cursor: &mut anchor_lang::AccountCursor<'info>, __ix_data: &[u8]) -> u64 {
                match (|| -> anchor_lang::Result<()> {
                use crate::#modname::*;
                #[cfg(not(feature="no-log-ix-name"))] anchor_lang::msg!(#log);
                let mut __data = __ix_data;
                #(#read_args)*
                let mut __bumps = <#accounts_ty as anchor_lang::Bumps>::Bumps::default();
                let mut __reallocs=std::collections::BTreeSet::new();
                if std::mem::size_of::<#accounts_ty>()>512 {
                    #[inline(never)] fn __load<'info,A:anchor_lang::Accounts<'info>,const RAW:bool>(program:&anchor_lang::Pubkey,cursor:&mut anchor_lang::AccountCursor<'info>,data:&[u8],bumps:&mut A::Bumps,reallocs:&mut std::collections::BTreeSet<anchor_lang::Pubkey>)->anchor_lang::Result<Box<A>> {
                        let mut storage=Box::<A>::new_uninit();
                        unsafe {A::__try_accounts_into_mode::<RAW>(program,cursor,data,bumps,reallocs,false,&mut storage)?;Ok(storage.assume_init())}
                    }
                    let mut __accounts=__load::<#accounts_ty,RAW>(__program_id,__cursor,__ix_data,&mut __bumps,&mut __reallocs)?;
                    #remaining_setup
                    let __result=({#[inline(never)] |context| #modname::#name(context,#(#call_args),*)})(anchor_lang::Context::new(__program_id, &mut *__accounts, #remaining_slice, __bumps))?;
                    #maybe_return
                    anchor_lang::Accounts::exit(&*__accounts, __program_id)
                } else {
                    let mut __accounts_storage=std::mem::MaybeUninit::<#accounts_ty>::uninit();
                    unsafe { <#accounts_ty as anchor_lang::Accounts>::__try_accounts_into_mode::<RAW>(__program_id, __cursor, __ix_data, &mut __bumps, &mut __reallocs, #direct #(#direct_args)*, &mut __accounts_storage)?; }
                    #slot_body
                }
                })() {Ok(())=>0,Err(error)=>__tare_error(error)}
            }
        });
        instructions.push(quote!{#(#cfgs)* #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)] pub struct #camel { #(#fields),* } #(#cfgs)* impl anchor_lang::Discriminator for #camel { const DISCRIMINATOR: &'static [u8] = #disc; }});
        instructions.push(quote! {#(#cfgs)* impl anchor_lang::InstructionData for #camel {}});
        let account_ident = if let Type::Path(p) = accounts_ty {
            &p.path.segments.last().unwrap().ident
        } else {
            return fail(accounts_ty, "CPI account type").into();
        };
        let cpi_module = format_ident!(
            "__cpi_client_accounts_{}",
            account_ident.to_string().to_snake_case()
        );
        cpi_exports.insert(
            cpi_module.to_string(),
            quote! {#(#cfgs)* pub use crate::#cpi_module::*;},
        );
        let invoke = quote! {anchor_lang::CpiInvoke::__invoke(&ctx.accounts,ctx.program,__data,ctx.signer_seeds,&ctx.remaining_accounts)?;};
        let serialize = if let Some(size) = wire_size.filter(|_| primitive_args) {
            quote! {
                if true #(#direct_args)* {
                    let mut __bytes=[0u8;#size];__bytes[..8].copy_from_slice(#disc);
                    anchor_lang::AnchorSerialize::serialize(&__instruction,&mut &mut __bytes[8..]).map_err(|_|anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
                    let __data=&__bytes[..];#invoke
                } else {
                    let mut __bytes=Vec::with_capacity(256);__bytes.extend_from_slice(#disc);
                    anchor_lang::AnchorSerialize::serialize(&__instruction,&mut __bytes).map_err(|_|anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
                    let __data=&__bytes[..];#invoke
                }
            }
        } else {
            quote! {
                let mut __bytes=Vec::with_capacity(256);__bytes.extend_from_slice(#disc);
                anchor_lang::AnchorSerialize::serialize(&__instruction,&mut __bytes).map_err(|_|anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
                let __data=&__bytes[..];#invoke
            }
        };
        let cpi_result = if unit_return {
            quote! {()}
        } else {
            quote! {anchor_lang::cpi::Return<#return_ty>}
        };
        let cpi_return = if unit_return {
            quote! {Ok(())}
        } else {
            quote! {Ok(anchor_lang::cpi::Return::new())}
        };
        cpi_methods.push(quote!{
            #(#cfgs)* pub fn #name<'a,'b,'c,'info>(ctx:anchor_lang::CpiContext<'a,'b,'c,'info,crate::#cpi_module::__Client<'info>>,#(#cpi_args),*)->anchor_lang::Result<#cpi_result> {
                let __instruction=instruction::#camel {#(#call_args),*};
                #serialize
                #cpi_return
            }
        });
    }
    let fallback_dispatch=match fallback {
        Some(name)=>quote!{{let __remaining=input.cursor_mode::<RAW>().remaining_storage();#modname::#name(program_id,__remaining.as_slice(),data)}},
        None=>quote!{Err(anchor_lang::error::ErrorCode::InstructionFallbackNotFound.into())},
    };
    let new_zero_semantics=TARE_ANCHOR_MAJOR>=1;
    let event_dispatch=if TARE_ANCHOR_MAJOR>=1 && cfg!(feature="event-cpi") {
        quote!{anchor_lang::event::dispatch_known(&mut input.cursor_mode::<RAW>(),&crate::EVENT_AUTHORITY_AND_BUMP.0)}
    }else{
        quote!{anchor_lang::event::dispatch(program_id,&mut input.cursor_mode::<RAW>())}
    };
    let program_type = format_ident!("{}", modname.to_string().to_camel_case());
    let cpi_exports: Vec<_> = cpi_exports.into_values().collect();
    let mut emitted_item=item.clone();
    if let Some((_,items))=&mut emitted_item.content {
        for part in items {
            if let syn::Item::Fn(fun)=part {
                if inline_handlers.contains(&fun.sig.ident) && !fun.attrs.iter().any(|attr|attr.path.is_ident("inline")) {
                    fun.attrs.push(syn::parse_quote!(#[inline]));
                }
            }
        }
    }

    let generated=quote!{
        use self::#modname::*;
        #emitted_item
        pub mod instruction { use super::*; use crate::#modname::*; use anchor_lang::borsh; #(#instructions)* }
        #[cfg(feature="cpi")]
        pub mod cpi {use super::*;use crate::#modname::*;pub use anchor_lang::cpi::Return;pub mod accounts {#(#cpi_exports)*}#(#cpi_methods)*}
        pub mod program { pub struct #program_type; impl anchor_lang::Id for #program_type { fn id()->anchor_lang::Pubkey { crate::ID } } }
        #(#wrappers)*
        #[cold]
        #[inline(never)]
        fn __tare_error(error:anchor_lang::Error)->u64 {
            error.log();
            u64::from(anchor_lang::solana_program::program_error::ProgramError::from(error))
        }
        fn __tare_dispatch<const RAW:bool>(program_id:&anchor_lang::Pubkey, input:anchor_lang::input::AccountInput, data:&[u8]) -> u64 {
            #[cfg(tare_rehearsal_defect)]
            if data == b"TAREBAD!" {
                anchor_lang::solana_program::log::sol_log("TARE REHEARSAL DEFECT: dispatch-log");
            }
            if *program_id != crate::ID { return __tare_error(anchor_lang::error::ErrorCode::DeclaredProgramIdMismatch.into()); }
            #(#dispatch)*
            if data.starts_with(&0x0a69e9a778bcf440u64.to_le_bytes()) {
                #[cfg(not(feature="no-idl"))]
                return match anchor_lang::idl::dispatch(program_id,&mut input.cursor_mode::<RAW>(),&data[8..],anchor_lang::error::Source {filename:file!(),line:line!()},cfg!(not(feature="no-log-ix-name")), #new_zero_semantics) {Ok(())=>0,Err(error)=>__tare_error(error)};
                #[cfg(feature="no-idl")]
                return __tare_error(anchor_lang::error::ErrorCode::IdlInstructionStub.into());
            }
            if data.starts_with(anchor_lang::event::EVENT_IX_TAG_LE) { return match #event_dispatch {Ok(())=>0,Err(error)=>__tare_error(error)}; }
            match #fallback_dispatch {Ok(())=>0,Err(error)=>__tare_error(error)}
        }
        #[cfg(not(feature="no-entrypoint"))]
        #[no_mangle] pub unsafe extern "C" fn entrypoint(input:*mut u8)->u64 {
            let (cursor,data,id)=anchor_lang::AccountCursor::from_input(input);
            __tare_dispatch::<true>(id,cursor.into_input(),data)
        }
        #[cfg(not(feature="no-entrypoint"))]
        anchor_lang::solana_program::custom_heap_default!();
        #[cfg(not(feature="no-entrypoint"))]
        anchor_lang::solana_program::custom_panic_default!();
    };
    let public_entry=if instruction_count==1 && TARE_ANCHOR_MAJOR>=1 {
        quote!{pub fn entry<'info>(program_id:&anchor_lang::Pubkey,accounts:&'info [anchor_lang::AccountInfo<'info>],data:&[u8])->anchor_lang::solana_program::entrypoint::ProgramResult {
            let header=[accounts.len(),accounts.as_ptr() as usize];let mut cursor=unsafe{anchor_lang::AccountCursor::from_sdk_header(&header)};let status=__tare_dispatch::<false>(program_id,&mut cursor,data);
            if status==0 {Ok(())}else{Err(anchor_lang::solana_program::program_error::ProgramError::from(status))}
        }}
    }else{
        quote!{pub fn entry<'info>(program_id:&anchor_lang::Pubkey,accounts:&'info [anchor_lang::AccountInfo<'info>],data:&[u8])->anchor_lang::solana_program::entrypoint::ProgramResult {
            let header=[accounts.len(),accounts.as_ptr() as usize];let cursor=unsafe{anchor_lang::AccountCursor::from_sdk_header(&header)};let status=__tare_dispatch::<false>(program_id,cursor.into_input(),data);
            if status==0 {Ok(())}else{Err(anchor_lang::solana_program::program_error::ProgramError::from(status))}
        }}
    };
    if instruction_count==1 && TARE_ANCHOR_MAJOR>=1 {
        let mut generated:syn::File=syn::parse2(generated).expect("valid generated program");
        for item in &mut generated.items {
            if let syn::Item::Fn(fun)=item {
                if fun.sig.ident=="__tare_dispatch" {
                    if let Some(syn::FnArg::Typed(arg))=fun.sig.inputs.iter_mut().nth(1) {
                        arg.ty=Box::new(syn::parse_quote!(&mut anchor_lang::AccountCursor));
                        arg.pat=Box::new(syn::parse_quote!(mut input));
                    }
                    fn eager_tokens(tokens:Tokens)->Tokens {
                        use proc_macro2::{Group,TokenTree};
                        let tokens=tokens.into_iter().collect::<Vec<_>>();let mut out=Tokens::new();let mut index=0;
                        while index<tokens.len() {
                            if index+3<tokens.len()
                                && matches!(&tokens[index],TokenTree::Ident(id) if id=="input")
                                && matches!(&tokens[index+1],TokenTree::Punct(p) if p.as_char()=='.')
                                && matches!(&tokens[index+2],TokenTree::Ident(id) if id=="cursor" || id=="cursor_mode") {
                                let mut end=index+3;
                                if matches!(&tokens[index+2],TokenTree::Ident(id) if id=="cursor_mode") {
                                    // Generated cursor_mode::<RAW>() has five generic tokens.
                                    if end+5<tokens.len()
                                        && matches!(&tokens[end],TokenTree::Punct(p) if p.as_char()==':')
                                        && matches!(&tokens[end+1],TokenTree::Punct(p) if p.as_char()==':')
                                        && matches!(&tokens[end+2],TokenTree::Punct(p) if p.as_char()=='<')
                                        && matches!(&tokens[end+3],TokenTree::Ident(id) if id=="RAW")
                                        && matches!(&tokens[end+4],TokenTree::Punct(p) if p.as_char()=='>') {end+=5;}
                                }
                                if matches!(&tokens[end],TokenTree::Group(group) if group.stream().is_empty()) {
                                    out.extend([tokens[index].clone()]);index=end+1;continue;
                                }
                            }
                            match &tokens[index] {
                                TokenTree::Group(group)=>{let mut mapped=Group::new(group.delimiter(),eager_tokens(group.stream()));mapped.set_span(group.span());out.extend([TokenTree::Group(mapped)]);}
                                token=>out.extend([token.clone()]),
                            }
                            index+=1;
                        }
                        out
                    }
                    let tokens=eager_tokens(fun.block.to_token_stream());
                    fun.block=Box::new(syn::parse2(tokens).expect("valid eager dispatch"));
                } else if fun.sig.ident=="entrypoint" {
                    fun.block=Box::new(syn::parse_quote!({
                        let (mut cursor,data,id)=anchor_lang::AccountCursor::from_input(input);
                        __tare_dispatch::<true>(id,&mut cursor,data)
                    }));
                }
            }
        }
        quote!(#generated #public_entry).into()
    } else {quote!(#generated #public_entry).into()}
}
#[proc_macro_derive(Accounts, attributes(account, instruction))]
pub fn accounts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_accounts(input) {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
#[derive(Default)]
struct Constraints {
    init: bool,
    if_needed: bool,
    realloc: Option<Expr>,
    realloc_payer: Option<Expr>,
    realloc_zero: Option<Expr>,
    seeds_program: Option<Expr>,
    zero: bool,
    close: Option<Expr>,
    mutable: bool,
    duplicate: bool,
    signer: bool,
    payer: Option<Expr>,
    space: Option<Expr>,
    seeds: Option<syn::ExprArray>,
    bump: Option<Option<Expr>>,
    has_one: Vec<(Expr, Option<Expr>)>,
    raw: Vec<(Expr, Option<Expr>)>,
    owner: Option<Expr>,
    address: Option<Expr>,
    custom_address: Option<Expr>,
    executable: bool,
    rent: Option<bool>,
    custom_mut: Option<Expr>,
    custom_signer: Option<Expr>,
    token_mint: Option<Expr>,
    token_authority: Option<Expr>,
    token_program: Option<Expr>,
    mint_decimals: Option<Expr>,
    mint_authority: Option<Expr>,
    mint_freeze_authority: Option<Expr>,
    mint_program: Option<Expr>,
    associated_mint: Option<Expr>,
    associated_authority: Option<Expr>,
    associated_program: Option<Expr>,
}
impl Constraints {
    fn spl(&self) -> bool {
        self.token_mint.is_some() || self.token_authority.is_some() || self.token_program.is_some()
        || self.mint_decimals.is_some() || self.mint_authority.is_some() || self.mint_freeze_authority.is_some() || self.mint_program.is_some()
        || self.associated_mint.is_some() || self.associated_authority.is_some() || self.associated_program.is_some()
    }
}
fn constraints(field: &syn::Field) -> syn::Result<Constraints> {
    let mut c = Constraints::default();
    for attr in &field.attrs {
        if attr.path.is_ident("account") {
            attr.parse_nested_meta(|m| {
                let key = m.path.to_token_stream().to_string().replace(' ', "");
                match key.as_str() {
                    "dup" if TARE_ANCHOR_MAJOR >= 1 => c.duplicate = true,
                    "init" => c.init = true,
                    "init_if_needed" => {
                        c.init = true;
                        c.if_needed = true;
                    }
                    "realloc" => c.realloc = Some(m.value()?.parse()?),
                    "realloc::payer" => c.realloc_payer = Some(m.value()?.parse()?),
                    "realloc::zero" => c.realloc_zero = Some(m.value()?.parse()?),
                    "seeds::program" => c.seeds_program = Some(m.value()?.parse()?),
                    "zero" => c.zero = true,
                    "close" => c.close = Some(m.value()?.parse()?),
                    "mut" => {
                        c.mutable = true;
                        if m.input.peek(Token![@]) {
                            m.input.parse::<Token![@]>()?;
                            c.custom_mut = Some(m.input.parse()?);
                        }
                    }
                    "signer" => {
                        c.signer = true;
                        if m.input.peek(Token![@]) {
                            m.input.parse::<Token![@]>()?;
                            c.custom_signer = Some(m.input.parse()?);
                        }
                    }
                    "payer" => c.payer = Some(m.value()?.parse()?),
                    "space" => c.space = Some(m.value()?.parse()?),
                    "seeds" => c.seeds = Some(m.value()?.parse()?),
                    "bump" => {
                        c.bump = Some(if m.input.peek(Token![=]) {
                            Some(m.value()?.parse()?)
                        } else {
                            None
                        })
                    }
                    "has_one" | "constraint" => {
                        let expression = m.value()?.parse()?;
                        let custom = if m.input.peek(Token![@]) {
                            m.input.parse::<Token![@]>()?;
                            Some(m.input.parse()?)
                        } else {
                            None
                        };
                        if key == "has_one" {
                            c.has_one.push((expression, custom));
                        } else {
                            c.raw.push((expression, custom));
                        }
                    }
                    "token::mint" => c.token_mint = Some(m.value()?.parse()?),
                    "token::authority" => c.token_authority = Some(m.value()?.parse()?),
                    "token::token_program" => c.token_program = Some(m.value()?.parse()?),
                    "mint::decimals" => c.mint_decimals = Some(m.value()?.parse()?),
                    "mint::authority" => c.mint_authority = Some(m.value()?.parse()?),
                    "mint::freeze_authority" => c.mint_freeze_authority = Some(m.value()?.parse()?),
                    "mint::token_program" => c.mint_program = Some(m.value()?.parse()?),
                    "associated_token::mint" => c.associated_mint = Some(m.value()?.parse()?),
                    "associated_token::authority" => c.associated_authority = Some(m.value()?.parse()?),
                    "associated_token::token_program" => c.associated_program = Some(m.value()?.parse()?),
                    "owner" => c.owner = Some(m.value()?.parse()?),
                    "address" => {
                        c.address = Some(m.value()?.parse()?);
                        if m.input.peek(Token![@]) {
                            m.input.parse::<Token![@]>()?;
                            c.custom_address = Some(m.input.parse()?);
                        }
                    },
                    "executable" => c.executable = true,
                    "rent_exempt" => {
                        let mode: Ident = m.value()?.parse()?;
                        c.rent = Some(mode == "enforce");
                    }
                    _ => return Err(m.error(format!("Tare does not implement {}", key))),
                }
                Ok(())
            })?;
        }
    }
    if c.init || c.zero {
        c.mutable = true;
        if c.init && c.seeds.is_none() && c.associated_mint.is_none() {
            c.signer = true;
        }
        if c.rent.is_none() {
            c.rent = Some(true);
        }
    }
    Ok(c)
}
fn derive_accounts(input: DeriveInput) -> syn::Result<Tokens> {
    let syn::Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "Tare Accounts requires a struct",
        ));
    };
    let name = &input.ident;
    let new_zero_semantics=TARE_ANCHOR_MAJOR>=1;
    let bumps_name = format_ident!("{}Bumps", name);
    let mut context_direct = true;
    let compact_close = data.fields.len() <= 6
        && data.fields.iter().any(|field| account_inner(&field.ty).is_ok_and(|(_, boxed, _)| boxed))
        && data.fields.iter().any(|field| constraints(field).is_ok_and(|c| c.close.is_some()))
        && data.fields.iter().all(|field| {
            constraints(field).is_ok_and(|c| !c.init && !c.zero && c.realloc.is_none())
                && account_inner(&field.ty).is_ok_and(|(_, _, ty)| is_leaf_account(ty))
        });
    let complex_accounts = data.fields.len() > 3
        || data.fields.iter().any(|f| {
            let c = constraints(f).unwrap_or_default();
            account_inner(&f.ty).map(|x| x.0 || !is_leaf_account(x.2)).unwrap_or(true) || c.realloc.is_some()
        });
    for field in &data.fields {
        let c = constraints(field)?;
        context_direct &= c.realloc.is_none();
        context_direct &=
            c.raw.is_empty() && c.owner.is_none() && c.address.is_none() && c.seeds.is_none() && !c.spl();
    }
    let mut generics = input.generics.clone();
    let lifetime = if let Some(lt) = generics.lifetimes().next() {
        lt.lifetime.clone()
    } else {
        let lt: syn::Lifetime = syn::parse_quote!('info);
        generics.params.insert(0, syn::parse_quote!('info));
        lt
    };
    let (impl_g, _, where_g) = generics.split_for_impl();
    let (_, ty_g, _) = input.generics.split_for_impl();
    let (bumps_impl_g, _, bumps_where_g) = input.generics.split_for_impl();
    let mut loads = Vec::new();
    let mut init_checks = Vec::new();
    let mut checks = Vec::new();
    let mut exits = Vec::new();
    let mut names = Vec::new();
    let mut bump_fields = Vec::new();
    let mut bump_defaults = Vec::new();
    let mut ix_args = Vec::new();
    let mut cpi_fields = Vec::new();
    let mut cpi_signers = Vec::new();
    let mut cpi_writable = Vec::new();
    let mut actual_metas = Vec::new();
    let mut prior_initializers: Vec<(Ident, bool)> = Vec::new();
    let mut dynamic_cpi = false;
    let mut duplicate_keys = Vec::new();
    let mut duplicate_names = Vec::new();
    let mut duplicate_pushes = Vec::new();
    let mut duplicate_nested = Vec::new();
    let mut client_metas = Vec::new();
    let mut client_infos = Vec::new();
    let metadata_fields=data.fields.iter().filter_map(|field| {
        let (_,_,ty)=account_inner(&field.ty).ok()?;
        let Type::Path(path)=ty else{return None};let kind=path.path.segments.last()?.ident.to_string();
        matches!(kind.as_str(),"Signer"|"Program"|"SystemAccount"|"UncheckedAccount"|"AccountInfo"|"Interface").then(||field.ident.as_ref().unwrap().to_string())
    }).collect::<std::collections::BTreeSet<_>>();
    let optional_fields = data
        .fields
        .iter()
        .filter_map(|f| match account_inner(&f.ty) {
            Ok((true, _, _)) => f.ident.as_ref(),
            _ => None,
        })
        .collect::<Vec<_>>();
    for attr in &input.attrs {
        if attr.path.is_ident("instruction") {
            let args = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::FnArg, Token![,]>::parse_terminated,
            )?;
            for a in args {
                let syn::FnArg::Typed(a) = a else {
                    return Err(syn::Error::new_spanned(
                        a,
                        "Tare instruction arguments require name: Type",
                    ));
                };
                let pattern = a.pat;
                let ty = a.ty;
                ix_args.push(quote!{#[allow(unused_variables)] let #pattern:#ty=anchor_lang::AnchorDeserialize::deserialize(&mut __ix_data).map_err(|_|anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;});
            }
        }
    }
    for field in &data.fields {
        let Some(id) = &field.ident else {
            return Err(syn::Error::new(
                field.span(),
                "Tare does not implement tuple Accounts",
            ));
        };
        let original_ty = &field.ty;
        let (optional, boxed, ty) = account_inner(original_ty)?;
        let text = id.to_string();
        let mut c = constraints(field)?;
        lower_constraint_fields(&mut c,&metadata_fields);
        let Type::Path(path) = ty else {
            return Err(syn::Error::new(
                ty.span(),
                "Tare does not implement this account type",
            ));
        };
        let kind = path.path.segments.last().unwrap().ident.to_string();
        if c.init && kind == "SystemAccount" {
            return Err(syn::Error::new_spanned(field,"Cannot use init on a SystemAccount"));
        }
        let known = [
            "Account",
            "Signer",
            "Program",
            "SystemAccount",
            "UncheckedAccount",
            "AccountInfo",
            "Interface",
            "InterfaceAccount",
            "AccountLoader",
            "Sysvar",
        ];
        if !known.contains(&kind.as_str()) {
            if matches!(
                kind.as_str(),
                "LazyAccount" | "ProgramData" | "Migration" | "AccountState"
            ) {
                return Err(syn::Error::new(
                    ty.span(),
                    format!("Tare does not implement {}", kind),
                ));
            }
            if optional
                || boxed
                || c.init
                || c.zero
                || c.mutable
                || c.signer
                || c.payer.is_some()
                || c.seeds.is_some()
                || c.owner.is_some()
                || c.close.is_some()
                || c.address.is_some()
                || c.rent.is_some()
                || c.executable
                || c.spl()
                || !c.has_one.is_empty()
            {
                return Err(syn::Error::new_spanned(
                    field,
                    "Tare does not implement this composite account constraint",
                ));
            }
            context_direct = false;
            dynamic_cpi = true;
            if TARE_ANCHOR_MAJOR >= 1 {
                duplicate_nested.push(quote!{for key in anchor_lang::DuplicateMutableAccountKeys::duplicate_mutable_account_keys(&#id) {
                    if __tare_duplicate_keys.contains(&key){return Err(anchor_lang::Error::from_static(&anchor_lang::error::ErrorDescriptor {code:2040,name:"ConstraintDuplicateMutableAccount",message:"A duplicate mutable account constraint was violated"}).with_account_key(key));}
                    __tare_duplicate_keys.push(key);
                }});
                duplicate_pushes.push(quote!{keys.extend(anchor_lang::DuplicateMutableAccountKeys::duplicate_mutable_account_keys(&self.#id));});
            }
            loads.push(quote!{let mut __nested=std::mem::MaybeUninit::<#ty>::uninit();unsafe{<#ty as anchor_lang::Accounts>::__try_accounts_into_mode::<RAW>(__program_id,__accounts,__data,&mut __bumps.#id,__reallocs,false,&mut __nested)?;}let #id=unsafe{__nested.assume_init()};});
            let mut bump_path=path.path.clone();let last=bump_path.segments.last_mut().unwrap();last.ident=format_ident!("{}Bumps",last.ident);last.arguments=PathArguments::None;
            bump_fields.push(quote!{pub #id:#bump_path});
            bump_defaults.push(quote!{#id:#bump_path::default()});
            exits.push(quote!{anchor_lang::Accounts::exit(&self.#id,program_id).map_err(|e|e.__tare_with_account_name_static(#text))?;});
            for (expr, custom) in &c.raw {
                let err = custom
                    .as_ref()
                    .map(|e| quote! {#e})
                    .unwrap_or(quote! {anchor_lang::ErrorCode::ConstraintRaw});
                checks.push(quote!{if !(#expr){return Err(anchor_lang::Error::from(#err).__tare_with_account_name_static(#text));}});
            }
            cpi_fields.push(quote! {pub #id:<#ty as anchor_lang::CpiAccounts<#lifetime>>::Cpi});
            actual_metas.push(quote!{metas.extend(anchor_lang::ToAccountMetas::to_account_metas(&self.#id,None));});
            client_metas.push(quote!{metas.extend(anchor_lang::ToAccountMetas::to_account_metas(&self.#id,None));});
            client_infos.push(
                quote! {infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.#id));},
            );
            names.push(id);
            continue;
        }
        if TARE_ANCHOR_MAJOR >= 1 && c.mutable && !c.duplicate && !(c.init && !c.if_needed)
            && matches!(kind.as_str(),"Account"|"InterfaceAccount") {
            duplicate_keys.push(if optional {quote!{#id.as_ref().map(|account|account.key())}}else{quote!{Some(#id.key())}});
            duplicate_names.push(text.clone());
            duplicate_pushes.push(if optional {quote!{if let Some(account)=&self.#id{keys.push(account.key());}}}else{quote!{keys.push(self.#id.key());}});
        }
        dynamic_cpi |= optional;
        if optional {
            cpi_fields.push(quote! {pub #id:Option<anchor_lang::AccountInfo<#lifetime>>});
        } else {
            cpi_fields.push(quote! {pub #id:anchor_lang::AccountInfo<#lifetime>});
        }
        let cpi_signer = c.signer || kind == "Signer";
        cpi_signers.push(cpi_signer);
        cpi_writable.push(c.mutable);
        let writable = c.mutable;
        let make_meta = if writable {
            quote! {anchor_lang::solana_program::instruction::AccountMeta::new}
        } else {
            quote! {anchor_lang::solana_program::instruction::AccountMeta::new_readonly}
        };
        let signer_override = if c.signer {
            quote! {Some(true)}
        } else {
            quote! {None}
        };
        if optional {
            actual_metas.push(quote!{if let Some(value)=&self.#id{metas.extend(anchor_lang::ToAccountMetas::to_account_metas(value,#signer_override));}else{metas.push(anchor_lang::solana_program::instruction::AccountMeta::new_readonly(crate::ID,false));}});
            client_metas.push(quote!{if let Some(value)=&self.#id{metas.push(#make_meta(*value.key,#cpi_signer));}else{metas.push(anchor_lang::solana_program::instruction::AccountMeta::new_readonly(crate::ID,false));}});
            client_infos.push(quote! {if let Some(value)=&self.#id{infos.push(value.clone());}});
        } else {
            actual_metas.push(quote!{metas.extend(anchor_lang::ToAccountMetas::to_account_metas(&self.#id,#signer_override));});
            client_metas.push(quote! {metas.push(#make_meta(*self.#id.key,#cpi_signer));});
            client_infos.push(quote! {infos.push(self.#id.clone());});
        }
        if c.init || c.zero {
            if optional {
                loads.push(quote! {let #id=__accounts.next_optional_mode::<RAW>(__program_id)?;});
            } else {
                loads.push(quote! {let #id=__accounts.next_mode::<RAW>()?;});
            }
        } else {
            let value = if kind == "Account" {
                let safe = c.mutable
                    && c.raw.is_empty()
                    && c.owner.is_none()
                    && c.address.is_none()
                    && c.seeds.is_none();
                quote! {anchor_lang::Account::__try_from_info(__raw,__direct && #safe)}
            } else {
                quote! {anchor_lang::FromAccountInfo::try_from_info(__raw)}
            };
            let wrap = if boxed {
                quote! {Box::new(value)}
            } else {
                quote! {value}
            };
            let load = if optional {
                quote! {match __accounts.next_optional_mode::<RAW>(__program_id).map_err(|e|e.__tare_with_account_name_static(#text))?{Some(__raw)=>{let value:#ty=#value.map_err(|e|e.__tare_with_account_name_static(#text))?;Some(#wrap)},None=>None}}
            } else {
                quote! {{let __raw=__accounts.next_mode::<RAW>().map_err(|e|e.__tare_with_account_name_static(#text))?;let value:#ty=#value.map_err(|e|e.__tare_with_account_name_static(#text))?;#wrap}}
            };
            if !optional {
                let direct_safe=kind=="Account" && c.mutable && c.raw.is_empty() && c.owner.is_none() && c.address.is_none() && c.seeds.is_none();
                let shared=if boxed {quote!{anchor_lang::__load_boxed_field::<#ty,RAW>(__accounts,#text)?}}else{quote!{
                    let mut __tare_field=std::mem::MaybeUninit::<#ty>::uninit();
                    unsafe{anchor_lang::__load_field_into::<#ty,RAW>(__accounts,#text,&mut __tare_field)?;}
                    unsafe{__tare_field.assume_init()}
                }};
                if compact_close {loads.push(quote!{let #id:#original_ty=#load;});}
                else if direct_safe {loads.push(quote!{let #id:#original_ty=if __direct {#load} else {#shared};});}
                else {loads.push(quote!{let #id:#original_ty={#shared};});}
            } else {
                let direct_safe=kind=="Account" && c.mutable && c.raw.is_empty() && c.owner.is_none() && c.address.is_none() && c.seeds.is_none();
                let shared=if boxed {quote!{anchor_lang::__load_optional_boxed_field::<#ty,RAW>(__accounts,__program_id,#text)?}}else{quote!{
                    let mut __tare_field=std::mem::MaybeUninit::<#original_ty>::uninit();
                    unsafe{anchor_lang::__load_optional_field_into::<#ty,RAW>(__accounts,__program_id,#text,&mut __tare_field)?;__tare_field.assume_init()}
                }};
                if direct_safe {loads.push(quote!{let #id:#original_ty=if __direct {#load}else{#shared};});}
                else {loads.push(quote!{let #id:#original_ty={#shared};});}
            }
        }
        let account_wrapper = if kind == "InterfaceAccount" {
            quote! {anchor_lang::InterfaceAccount}
        } else if kind == "AccountLoader" {
            quote! {anchor_lang::AccountLoader}
        } else {
            quote! {anchor_lang::Account}
        };
        let mut field_checks = Vec::new();
        if c.rent == Some(true) || c.init {
            field_checks.push(quote! {let __anchor_rent=anchor_lang::rent::get()?;});
        }
        if c.zero {
            let Type::Path(tp) = ty else { unreachable!() };
            let PathArguments::AngleBracketed(args) = &tp.path.segments.last().unwrap().arguments
            else {
                return Err(syn::Error::new_spanned(ty, "zero requires Account<T>"));
            };
            let Some(GenericArgument::Type(account_ty)) = args.args.last() else {
                return Err(syn::Error::new_spanned(ty, "zero requires Account<T>"));
            };
            let unique=prior_initializers.iter().map(|(other,is_optional)|if *is_optional {quote!{if let Some(other)=&#other{if #id.key()==other.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintZero).__tare_with_account_name_static(#text));}}}}else{quote!{if #id.key()==#other.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintZero).__tare_with_account_name_static(#text));}}});
            field_checks.push(quote!{
                let #id:#ty={
                    let __tare_zero_bytes=#id.try_borrow_data()?;
                    if #new_zero_semantics && __tare_zero_bytes.len()< <#account_ty as anchor_lang::Discriminator>::DISCRIMINATOR.len() {return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::AccountDidNotDeserialize).__tare_with_account_name_static(#text));}
                    if __tare_zero_bytes[..<#account_ty as anchor_lang::Discriminator>::DISCRIMINATOR.len()].iter().any(|b|*b!=0) {return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintZero).__tare_with_account_name_static(#text));}
                    #(#unique)*
                    drop(__tare_zero_bytes);
                    #account_wrapper::__try_from_unchecked(#id,false).map_err(|e|e.__tare_with_account_name_static(#text))?
                };
            });
        }
        if let Some(space) = &c.realloc {
            let payer = c
                .realloc_payer
                .as_ref()
                .ok_or_else(|| syn::Error::new_spanned(field, "realloc requires realloc::payer"))?;
            let zero = c
                .realloc_zero
                .as_ref()
                .ok_or_else(|| syn::Error::new_spanned(field, "realloc requires realloc::zero"))?;
            let payer_check = optional_check(payer, &optional_fields);
            let system_check = optional_check(&syn::parse_quote!(system_program), &optional_fields);
            let self_payer_check=if TARE_ANCHOR_MAJOR>=1 {
                quote! {
                    if #payer.key()==#id.key() {
                        return Err(anchor_lang::Error::from(anchor_lang::solana_program::program_error::ProgramError::InvalidArgument)
                            .__tare_with_account_name_static(#text).with_pubkeys((#payer.key(),#id.key())));
                    }
                }
            } else {quote!{}};
            field_checks.push(quote!{
                ({#[inline(never)] ||->anchor_lang::Result<()> {
                    if __reallocs.contains(&#id.key()){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::AccountDuplicateReallocs).__tare_with_account_name_static(#text));}
                    let __rent=anchor_lang::rent::get()?;
                    let __info=#id.to_account_info();
                    let __minimum=anchor_lang::rent::minimum_balance(&__rent,#space);
                    let __delta=(std::convert::TryInto::<isize>::try_into(#space).unwrap()).checked_sub(std::convert::TryInto::try_into(__info.data_len()).unwrap()).unwrap();
                    if __delta!=0 {
                        #payer_check
                        #self_payer_check
                        if __delta>0 {
                            #system_check
                            if std::convert::TryInto::<usize>::try_into(__delta).unwrap()>10240 {return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::AccountReallocExceedsLimit).__tare_with_account_name_static(#text));}
                            if __minimum>__info.lamports() {
                                anchor_lang::system_program::transfer(anchor_lang::CpiContext::new(system_program.to_account_info(),anchor_lang::system_program::Transfer{from:#payer.to_account_info(),to:__info.clone()}),__minimum.checked_sub(__info.lamports()).unwrap())?;
                            }
                        }else{
                            let amount=__info.lamports().checked_sub(__minimum).unwrap();
                            **#payer.to_account_info().lamports.borrow_mut()=#payer.to_account_info().lamports().checked_add(amount).unwrap();
                            **__info.lamports.borrow_mut()=__info.lamports().checked_sub(amount).unwrap();
                        }
                        __info.realloc(#space,#zero)?;
                        __reallocs.insert(#id.key());
                    }
                    Ok(())
                }})()?;
            });
        }
        let mut signer_seeds = quote! {&[]};
        if let Some(seeds) = &c.seeds {
            let elems = &seeds.elems;
            let seed_elements: Vec<_> = elems.iter().collect();
            if seed_elements.len() > 15 {
                return Err(syn::Error::new(
                    seeds.span(),
                    "Tare seeds exceed the Solana limit",
                ));
            }

            let seeds_program = c
                .seeds_program
                .as_ref()
                .map(|e| quote! {&{ use anchor_lang::Key as _; (#e).key() }})
                .unwrap_or(quote! {__program_id});
            let bump_var = format_ident!("__{}_bump", id);
            if c.init || matches!(c.bump,Some(None)) {
                if optional {bump_fields.push(quote!{pub #id:Option<u8>});bump_defaults.push(quote!{#id:None});}
                else{bump_fields.push(quote!{pub #id:u8});bump_defaults.push(quote!{#id:u8::MAX});}
            }
            let save_bump = if optional {
                quote! {__bumps.#id=Some(#bump_var);}
            } else {
                quote! {__bumps.#id=#bump_var;}
            };
            if c.init || matches!(c.bump, Some(None)) {
                let explicit_check = if let Some(Some(bump)) = &c.bump {
                    quote! {if #bump_var!=#bump{return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintSeeds).__tare_with_account_name_static(#text).with_values((#bump_var,#bump)));}}
                } else {
                    quote! {}
                };
                field_checks.push(quote!{
                    let (__pda,#bump_var)=anchor_lang::Pubkey::find_program_address(&[#(#seed_elements),*],#seeds_program);
                    #save_bump
                    if #id.key()!=__pda {return Err(anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::ConstraintSeeds).__tare_with_account_name_static(#text).with_pubkeys((#id.key(),__pda)));}
                    #explicit_check
                });
            } else if let Some(Some(bump)) = &c.bump {
                field_checks.push(quote!{
                    let #bump_var=#bump;
                    let __pda=anchor_lang::Pubkey::create_program_address(&[#(#seed_elements,)* &[#bump_var]],#seeds_program).map_err(|_|anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::ConstraintSeeds).__tare_with_account_name_static(#text))?;
                    if #id.key()!=__pda {return Err(anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::ConstraintSeeds).__tare_with_account_name_static(#text).with_pubkeys((#id.key(),__pda)));}
                });
            } else {
                return Err(syn::Error::new(
                    seeds.span(),
                    "Tare requires bump with seeds",
                ));
            }
            signer_seeds = quote! {&[#(#seed_elements,)* &[#bump_var]]};
        }
        if c.init && c.spl() {
            let code = spl_init(&c,id,ty,&account_wrapper,&text,&signer_seeds,&optional_fields)?;
            field_checks.push(quote!{
                let mut __tare_spl_output=std::mem::MaybeUninit::<#ty>::uninit();
                ({#[inline(never)] ||->anchor_lang::Result<()>{#code}})()?;
                let #id:#ty=unsafe {__tare_spl_output.assume_init()};
            });
        } else if c.init {
            let payer = c
                .payer
                .as_ref()
                .ok_or_else(|| syn::Error::new(field.span(), "init requires payer"))?;
            let space = c
                .space
                .as_ref()
                .ok_or_else(|| syn::Error::new_spanned(field, "init requires space"))?;
            let owner = c
                .owner
                .as_ref()
                .map(|o| quote! {&#o})
                .unwrap_or(quote! {__program_id});
            let direct_init =
                c.raw.is_empty() && c.owner.is_none() && c.address.is_none() && c.seeds.is_none() && !c.spl();
            let system_check = optional_check(&syn::parse_quote!(system_program), &optional_fields);
            let payer_check = optional_check(payer, &optional_fields);
            let if_needed = c.if_needed;
            let checked = if kind == "Account" {
                quote! {anchor_lang::Account::__try_from_info(#id,__direct && #direct_init)}
            } else {
                quote! {anchor_lang::FromAccountInfo::try_from_info(#id)}
            };
            let unchecked = if matches!(kind.as_str(),"UncheckedAccount"|"AccountInfo") {
                quote!{anchor_lang::FromAccountInfo::try_from_info(#id)}
            }else{quote!{#account_wrapper::__try_from_unchecked(#id,__direct && #direct_init)}};
            let init_code = quote! {
                    #system_check
                    let __actual=#id;
                    let __space=#space;
                    let value:#ty=if !#if_needed || __actual.owner_ref()==&anchor_lang::solana_program::system_program::ID {
                        #payer_check
                        if #id.lamports()!=0 && #payer.key()==#id.key(){return Err(anchor_lang::error!(anchor_lang::ErrorCode::TryingToInitPayerAsProgramAccount).with_pubkeys((#payer.key(),#id.key())));}
                        anchor_lang::cpi::create_account(#payer.__tare_info(),#id,system_program.__tare_info(),#owner,__space,&__anchor_rent,#signer_seeds)?;
                        #unchecked.map_err(|e|e.__tare_with_account_name_static(#text))?
                    }else{#checked.map_err(|e|e.__tare_with_account_name_static(#text))?};
                    if #if_needed {
                        if __space!=__actual.data_len(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintSpace).__tare_with_account_name_static(#text).with_values((__space,__actual.data_len())));}
                        if __actual.owner_ref()!=#owner{return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintOwner).__tare_with_account_name_static(#text).with_pubkeys((*__actual.owner_ref(),*#owner)));}
                        if value.__tare_info().lamports()<anchor_lang::rent::minimum_balance(&__anchor_rent,__space){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintRentExempt).__tare_with_account_name_static(#text));}
                    }
                    Ok(value)
            };
            if complex_accounts {
                field_checks.push(quote!{let #id:#ty=({#[inline(never)] ||->anchor_lang::Result<#ty>{#init_code}})()?;});
            } else {
                field_checks
                    .push(quote! {let #id:#ty=(||->anchor_lang::Result<#ty>{#init_code})()?;});
            }
        }
        if !c.init {
            if let Some(code) = associated_check(&c,id,&text,&optional_fields)? { field_checks.push(code); }
        }
        if c.mutable {
            let err = c
                .custom_mut
                .as_ref()
                .map(|e| quote! {#e})
                .unwrap_or(quote! {anchor_lang::error::ErrorCode::ConstraintMut});
            field_checks.push(quote!{if !#id.__tare_info().is_writable() {return Err(anchor_lang::error::Error::from(#err).__tare_with_account_name_static(#text));}});
        }
        if c.signer {
            let err = c
                .custom_signer
                .as_ref()
                .map(|e| quote! {#e})
                .unwrap_or(quote! {anchor_lang::error::ErrorCode::ConstraintSigner});
            field_checks.push(quote!{if !#id.__tare_info().is_signer() {return Err(anchor_lang::error::Error::from(#err).__tare_with_account_name_static(#text));}});
        }
        for (target, custom) in &c.has_one {
            let Expr::Path(target_path) = target else {
                return Err(syn::Error::new(
                    target.span(),
                    "Tare has_one requires a field name",
                ));
            };
            let target_id = &target_path.path.segments.last().unwrap().ident;
            let err = custom
                .as_ref()
                .map(|e| quote! {#e})
                .unwrap_or(quote! {anchor_lang::error::ErrorCode::ConstraintHasOne});
            let access = if kind == "AccountLoader" {
                quote! {#id.load()?.#target_id}
            } else {
                quote! {#id.#target_id}
            };
            let optional_target = optional_check(target, &optional_fields);
            field_checks.push(quote!{{#optional_target if #access!=#target.key() {return Err(anchor_lang::error::Error::from(#err).__tare_with_account_name_static(#text).with_pubkeys((#access,#target.key())));}}});
        }
        for (expr, custom) in &c.raw {
            let err = custom
                .as_ref()
                .map(|e| quote! {#e})
                .unwrap_or(quote! {anchor_lang::error::ErrorCode::ConstraintRaw});
            field_checks.push(quote!{if !(#expr) {return Err(anchor_lang::error::Error::from(#err).__tare_with_account_name_static(#text));}});
        }
        if !c.init {
            if let Some(owner) = &c.owner {
                field_checks.push(quote!{if *#id.__tare_info().owner_ref()!=#owner {return Err(anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::ConstraintOwner).__tare_with_account_name_static(#text).with_pubkeys((*#id.__tare_info().owner_ref(),#owner)));}});
            }
        }
        if c.rent == Some(true) {
            field_checks.push(quote!{if !anchor_lang::rent::is_exempt(&__anchor_rent,#id.__tare_info().lamports(),#id.__tare_info().data_len()) {return Err(anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::ConstraintRentExempt).__tare_with_account_name_static(#text));}});
        }
        if c.executable {
            field_checks.push(quote!{if !#id.__tare_info().executable() {return Err(anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::ConstraintExecutable).__tare_with_account_name_static(#text));}});
        }
        if let Some(destination) = &c.close {
            let target_check = optional_check(destination, &optional_fields);
            field_checks.push(quote!{{#target_check if #id.key()==#destination.key() {return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintClose).__tare_with_account_name_static(#text));}}});
        }
        if let Some(address) = &c.address {
            let error = c.custom_address.as_ref().map(|error| quote!{#error})
                .unwrap_or_else(|| quote!{anchor_lang::error::ErrorCode::ConstraintAddress});
            field_checks.push(quote!{{let actual=#id.key();let expected=#address;if actual!=expected {return Err(anchor_lang::error::Error::from(#error).__tare_with_account_name_static(#text).with_pubkeys((actual,expected)));}}});
        }
        if !c.init { field_checks.extend(spl_checks(&c,id,&optional_fields)); }
        let rent_check = if c.rent == Some(true) || c.init {
            Some(field_checks.remove(0))
        } else {
            None
        };
        let all_checks = if optional {
            if c.init || c.zero {
                let wrap = if boxed {
                    quote! {Box::new(#id)}
                } else {
                    quote! {#id}
                };
                quote! {let #id:#original_ty=if let Some(#id)=#id{#(#field_checks)*Some(#wrap)}else{None};}
            } else {
                quote! {if let Some(#id)=&#id{#(#field_checks)*}}
            }
        } else if (c.init || c.zero) && boxed {
            quote! {#(#field_checks)*let #id=Box::new(#id);}
        } else {
            quote! {#(#field_checks)*}
        };
        let all_checks = if complex_accounts && !compact_close {
            if c.init || c.zero {
                quote! {let #id:#original_ty=({#[inline(never)] ||->anchor_lang::Result<#original_ty>{#all_checks Ok(#id)}})()?;}
            } else {
                quote! {({#[inline(never)] ||->anchor_lang::Result<()>{#all_checks Ok(())}})()?;}
            }
        } else {
            all_checks
        };
        if c.init {
            init_checks.push(quote! {#rent_check #all_checks});
        } else {
            checks.push(quote! {#rent_check #all_checks});
        }
        if let Some(destination) = &c.close {
            let target_check = optional_check(destination, &optional_fields);
            let source = if boxed {
                quote! {&**value}
            } else {
                quote! {value}
            };
            let body = quote! {let #destination=&self.#destination;#target_check anchor_lang::AccountsClose::close(#source,#destination.to_account_info()).map_err(|e|e.__tare_with_account_name_static(#text))?;};
            if optional {
                exits.push(quote! {if let Some(value)=&self.#id{#body}});
            } else {
                exits.push(quote! {{let value=&self.#id;#body}});
            }
        } else if matches!(
            kind.as_str(),
            "Account" | "InterfaceAccount" | "AccountLoader"
        ) && c.mutable
        {
            if optional {
                exits.push(quote!{if let Some(value)=&self.#id{value.exit(program_id).map_err(|e|e.__tare_with_account_name_static(#text))?;}});
            } else {
                exits.push(
                    quote! {self.#id.exit(program_id).map_err(|e|e.__tare_with_account_name_static(#text))?;},
                );
            }
        }
        if c.init || c.zero {
            prior_initializers.push((id.clone(), optional));
        }
        names.push(id);
    }
    let cpi_module = format_ident!("__cpi_client_accounts_{}", name.to_string().to_snake_case());
    let count = names.len();
    let cpi_default=if count==0 {quote!{#[derive(Default)]}}else{quote!{}};
    let (cpi_gen, cpi_type) = if count == 0 {
        (quote! {}, quote! {#name})
    } else {
        (quote! {<#lifetime>}, quote! {#name<#lifetime>})
    };
    let cpi_alias = if count == 0 {
        quote! {#name}
    } else {
        quote! {#name<'__cpi>}
    };
    let account_inline = if complex_accounts && !compact_close {
        quote! {#[inline(never)]}
    } else {
        quote! {#[inline(always)]}
    };
    let client_infos_body = quote! {let mut infos=Vec::new();#(#client_infos)*infos};
    let client_metas_body = quote! {let mut metas=Vec::new();#(#client_metas)*metas};
    let duplicate_count=duplicate_keys.len();
    let duplicate_tests=duplicate_names.iter().enumerate().map(|(index,name)|quote!{
        if let Some(key)=__tare_direct_keys[#index] {
            if __tare_direct_keys[..#index].contains(&Some(key)) {
                return Err(anchor_lang::Error::from_static(&anchor_lang::error::ErrorDescriptor {code:2040,name:"ConstraintDuplicateMutableAccount",message:"A duplicate mutable account constraint was violated"}).__tare_with_account_name_static(#name));
            }
        }
    }).collect::<Vec<_>>();
    let duplicates=if duplicate_count==0 && duplicate_nested.is_empty(){quote!{}}else{
        let nested=if duplicate_nested.is_empty(){quote!{}}else{quote!{
            let mut __tare_duplicate_keys=__tare_direct_keys.iter().flatten().copied().collect::<Vec<_>>();
            #(#duplicate_nested)*
        }};
        quote!{{let __tare_direct_keys:[Option<anchor_lang::Pubkey>;#duplicate_count]=[#(#duplicate_keys),*];#(#duplicate_tests)* #nested}}
    };
    let duplicate_impl=if TARE_ANCHOR_MAJOR>=1 {quote!{
        impl #bumps_impl_g anchor_lang::DuplicateMutableAccountKeys for #name #ty_g #bumps_where_g {
            fn duplicate_mutable_account_keys(&self)->Vec<anchor_lang::Pubkey>{use anchor_lang::Key;let mut keys=Vec::new();#(#duplicate_pushes)* keys}
        }
    }}else{quote!{}};
    let invoke_body = if dynamic_cpi {
        quote! {let mut infos=anchor_lang::ToAccountInfos::to_account_infos(self);let mut metas=anchor_lang::ToAccountMetas::to_account_metas(self,None);for info in remaining{infos.push(info.clone());metas.extend(anchor_lang::ToAccountMetas::to_account_metas(info,None));}infos.push(program.clone());anchor_lang::cpi::invoke_dynamic_with_id(program.key,&infos,&metas,data,signers)}
    } else {
        quote! {anchor_lang::cpi::invoke_generated::<#count>(program,[#(self.#names.clone()),*],[#(#cpi_signers),*],[#(#cpi_writable),*],data,signers,remaining)}
    };
    let generated=quote! {
        pub(crate) mod #cpi_module {
            use super::*;
            #cpi_default pub struct #name #cpi_gen {#(#cpi_fields),*}
            pub type __Client<'__cpi>=#cpi_alias;
            impl #impl_g anchor_lang::ToAccountInfos<#lifetime> for #cpi_type #where_g {
                fn to_account_infos(&self)->Vec<anchor_lang::AccountInfo<#lifetime>> {#client_infos_body}
            }
            impl #impl_g anchor_lang::ToAccountMetas for #cpi_type #where_g {
                fn to_account_metas(&self,_is_signer:Option<bool>)->Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    #client_metas_body
                }
            }
            impl #impl_g anchor_lang::CpiInvoke<#lifetime> for #cpi_type #where_g {
                fn __invoke(&self,program:anchor_lang::AccountInfo<#lifetime>,data:&[u8],signers:&[&[&[u8]]],remaining:&[anchor_lang::AccountInfo<#lifetime>])->anchor_lang::Result<()> {
                    #invoke_body
                }
            }
        }
        #duplicate_impl
        impl #impl_g anchor_lang::CpiAccounts<#lifetime> for #name #ty_g #where_g {type Cpi=#cpi_module::#cpi_type;}
        impl #impl_g anchor_lang::ToAccountInfos<#lifetime> for #name #ty_g #where_g {fn to_account_infos(&self)->Vec<anchor_lang::AccountInfo<#lifetime>> {let mut infos=Vec::new();#(infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.#names));)*infos}}
        impl #impl_g anchor_lang::ToAccountMetas for #name #ty_g #where_g {fn to_account_metas(&self,_is_signer:Option<bool>)->Vec<anchor_lang::solana_program::instruction::AccountMeta> {let mut metas=Vec::new();#(#actual_metas)*metas}}
        #[derive(Debug)] pub struct #bumps_name {#(#bump_fields),*}
        impl Default for #bumps_name {fn default()->Self{Self{#(#bump_defaults),*}}}
        impl #bumps_impl_g anchor_lang::Bumps for #name #ty_g #bumps_where_g {type Bumps=#bumps_name;}
        impl #impl_g anchor_lang::Accounts<#lifetime> for #name #ty_g #where_g {
            #account_inline fn try_accounts(__program_id:&anchor_lang::Pubkey,__accounts:&mut anchor_lang::AccountCursor<#lifetime>,__data:&[u8],__bumps:&mut #bumps_name,__reallocs:&mut std::collections::BTreeSet<anchor_lang::Pubkey>,__direct:bool)->anchor_lang::Result<Self> {
                let mut __tare_output=std::mem::MaybeUninit::<Self>::uninit();
                unsafe {Self::__try_accounts_into(__program_id,__accounts,__data,__bumps,__reallocs,__direct,&mut __tare_output)?;Ok(__tare_output.assume_init())}
            }
            #account_inline unsafe fn __try_accounts_into(__program_id:&anchor_lang::Pubkey,__accounts:&mut anchor_lang::AccountCursor<#lifetime>,__data:&[u8],__bumps:&mut #bumps_name,__reallocs:&mut std::collections::BTreeSet<anchor_lang::Pubkey>,__direct:bool,__tare_output:&mut std::mem::MaybeUninit<Self>)->anchor_lang::Result<()> {Self::__try_accounts_into_mode::<false>(__program_id,__accounts,__data,__bumps,__reallocs,__direct,__tare_output)}
            #account_inline unsafe fn __try_accounts_into_mode<const RAW:bool>(__program_id:&anchor_lang::Pubkey,__accounts:&mut anchor_lang::AccountCursor<#lifetime>,__data:&[u8],__bumps:&mut #bumps_name,__reallocs:&mut std::collections::BTreeSet<anchor_lang::Pubkey>,__direct:bool,__tare_output:&mut std::mem::MaybeUninit<Self>)->anchor_lang::Result<()> {use anchor_lang::{ToAccountInfo,Key};use anchor_lang::solana_program::sysvar::Sysvar as _;let __direct=__direct && #context_direct && __accounts.typed_prefix_is_unique_mode::<RAW>(#count);let mut __ix_data=__data;#(#ix_args)* #(#loads)* #(#init_checks)* #duplicates #(#checks)* let __tare_pointer=__tare_output.as_mut_ptr();#(std::ptr::addr_of_mut!((*__tare_pointer).#names).write(#names);)*Ok(())}
            #[inline(always)] fn exit(&self,program_id:&anchor_lang::Pubkey)->anchor_lang::Result<()> {#(#exits)* Ok(())}
        }
    };
    Ok(generated)
}
fn lower_constraint_fields(c:&mut Constraints,accounts:&std::collections::BTreeSet<String>) {
    fn lower(expr:&mut Expr,accounts:&std::collections::BTreeSet<String>) {
        fn unsafe_scope(tokens:Tokens)->bool {tokens.into_iter().any(|token|match token {proc_macro2::TokenTree::Ident(id)=>id=="let"||id=="match"||id=="for"||id=="self",proc_macro2::TokenTree::Punct(p)=>p.as_char()=='|'||p.as_char()=='!',proc_macro2::TokenTree::Group(g)=>unsafe_scope(g.stream()),_=>false})}
        let tokens=expr.to_token_stream();
        if !unsafe_scope(tokens.clone()) {*expr=syn::parse2(lower_validation_metadata(tokens,accounts)).expect("lowered constraint expression");}
    }
    for value in [&mut c.realloc,&mut c.realloc_payer,&mut c.realloc_zero,&mut c.seeds_program,&mut c.close,&mut c.payer,&mut c.space,&mut c.owner,&mut c.address,&mut c.custom_address,&mut c.custom_mut,&mut c.custom_signer,&mut c.token_mint,&mut c.token_authority,&mut c.token_program,&mut c.mint_decimals,&mut c.mint_authority,&mut c.mint_freeze_authority,&mut c.mint_program,&mut c.associated_mint,&mut c.associated_authority,&mut c.associated_program] {
        if let Some(value)=value {lower(value,accounts);}
    }
    if let Some(Some(value))=&mut c.bump {lower(value,accounts);}
    if let Some(seeds)=&mut c.seeds {for value in &mut seeds.elems {lower(value,accounts);}}
    for (value,error) in c.raw.iter_mut().chain(c.has_one.iter_mut()) {lower(value,accounts);if let Some(error)=error {lower(error,accounts);}}
}
fn lower_validation_metadata(tokens:Tokens,accounts:&std::collections::BTreeSet<String>)->Tokens {
    use proc_macro2::{TokenTree,Group};
    fn has_user_binding(tokens:Tokens)->bool {
        for token in tokens {
            match token {
                TokenTree::Ident(id) if id=="let"=>return true,
                TokenTree::Punct(p) if p.as_char()=='|'=>return true,
                TokenTree::Group(group) if has_user_binding(group.stream())=>return true,
                _=>{}
            }
        }
        false
    }
    let list=tokens.into_iter().collect::<Vec<_>>();let mut out=Tokens::new();let mut index=0;
    while index<list.len() {
        if index+2<list.len() {
            if let (TokenTree::Ident(account),TokenTree::Punct(dot),TokenTree::Ident(field))=(&list[index],&list[index+1],&list[index+2]) {
                let selected_by_dot=matches!(index.checked_sub(1).and_then(|i|list.get(i)),Some(TokenTree::Punct(p)) if p.as_char()=='.');
                if accounts.contains(&account.to_string()) && dot.as_char()=='.' && !selected_by_dot {
                    let method=match field.to_string().as_str(){"key"=>Some("key_ref"),"owner"=>Some("owner_ref"),"is_signer"=>Some("is_signer"),"is_writable"=>Some("is_writable"),"executable"=>Some("executable"),_=>None};
                    let called=matches!(list.get(index+3),Some(TokenTree::Group(group)) if group.delimiter()==proc_macro2::Delimiter::Parenthesis);
                    if let Some(method)=method.filter(|_|!called) {
                        let method=Ident::new(method,field.span());out.extend(quote!(#account.__tare_info().#method()));index+=3;continue;
                    }
                }
            }
        }
        match &list[index] {TokenTree::Group(group)=>{let inner=group.stream();let inner=if has_user_binding(inner.clone()){inner}else{lower_validation_metadata(inner,accounts)};let mut mapped=Group::new(group.delimiter(),inner);mapped.set_span(group.span());out.extend([TokenTree::Group(mapped)]);},value=>out.extend([value.clone()])}
        index+=1;
    }
    out
}
#[proc_macro_derive(InitSpace, attributes(max_len, borsh))]
pub fn init_space(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match space_derive(&input) {Ok(value)=>value.into(),Err(error)=>error.to_compile_error().into()}
}
fn space_derive(input:&DeriveInput)->syn::Result<Tokens> {
    let name=&input.ident;let (impl_generics,ty_generics,where_clause)=input.generics.split_for_impl();
    let size=match &input.data {
        syn::Data::Struct(data)=>space_fields(&data.fields)?,
        syn::Data::Enum(data)=>{
            let sizes=data.variants.iter().map(|v|space_fields(&v.fields)).collect::<syn::Result<Vec<_>>>()?;
            let max=sizes.into_iter().rev().fold(quote!{0usize},|right,left|quote!{{let left=#left;let right=#right;if left>right{left}else{right}}});
            quote!{1usize + #max}
        }
        syn::Data::Union(_)=>return Err(syn::Error::new_spanned(input,"Tare does not implement InitSpace for unions")),
    };
    Ok(quote!{impl #impl_generics anchor_lang::Space for #name #ty_generics #where_clause {const INIT_SPACE:usize=#size;}})
}
fn space_fields(fields:&syn::Fields)->syn::Result<Tokens> {
    let mut sizes=Vec::new();
    for field in fields {
        let mut skipped=false;
        if TARE_ANCHOR_MAJOR>=1 {
            for attr in field.attrs.iter().filter(|a|a.path.is_ident("borsh")) {
                if let syn::Meta::List(meta)=attr.parse_meta()? {
                    for value in meta.nested {
                        match value {
                            syn::NestedMeta::Meta(syn::Meta::Path(path)) if path.is_ident("skip")=>skipped=true,
                            syn::NestedMeta::Meta(syn::Meta::NameValue(value)) if value.path.is_ident("serialize_with")||value.path.is_ident("deserialize_with")=>
                                return Err(syn::Error::new_spanned(value,"InitSpace does not support custom Borsh serialization")),
                            _=>{}
                        }
                    }
                }
            }
        }
        if skipped {sizes.push(quote!{0usize});continue}
        let mut lengths=None;
        if let Some(attr)=field.attrs.iter().find(|a|a.path.is_ident("max_len")) {
            let args=attr.parse_args_with(syn::punctuated::Punctuated::<Expr,Token![,]>::parse_terminated)?;
            let mut parsed=std::collections::VecDeque::new();
            for arg in args {
                parsed.push_back(match &arg {
                    Expr::Path(path) if TARE_ANCHOR_MAJOR>=1||path.path.segments.len()==1=>quote!{(#path as usize)},
                    Expr::Lit(lit) if matches!(lit.lit,syn::Lit::Int(_))=>if TARE_ANCHOR_MAJOR>=1{quote!{(#lit as usize)}}else{quote!{#lit}},
                    _=>return Err(syn::Error::new_spanned(arg,"max_len only accepts integer literals, identifiers, or paths")),
                });
            }
            lengths=Some(parsed);
        }
        sizes.push(space_type(&field.ty,&mut lengths)?);
    }
    Ok(quote!{0usize #(+ #sizes)*})
}
fn space_type(ty:&Type,lengths:&mut Option<std::collections::VecDeque<Tokens>>)->syn::Result<Tokens> {
    match ty {
        Type::Array(array)=>{let count=&array.len;let size=space_type(&array.elem,lengths)?;Ok(quote!{(#count * #size)})}
        Type::Tuple(tuple)=>{let sizes=tuple.elems.iter().map(|ty|space_type(ty,lengths)).collect::<syn::Result<Vec<_>>>()?;Ok(quote!{(0usize #(+ #sizes)*)})}
        Type::Path(path)=>{
            let segment=path.path.segments.last().unwrap();let name=segment.ident.to_string();
            let first=if let PathArguments::AngleBracketed(args)=&segment.arguments{args.args.iter().find_map(|a|if let GenericArgument::Type(ty)=a{Some(ty)}else{None})}else{None};
            Ok(match name.as_str(){
                "bool"|"u8"|"i8"=>quote!{1usize},"u16"|"i16"=>quote!{2usize},
                "u32"|"i32"|"f32"=>quote!{4usize},"u64"|"i64"|"f64"=>quote!{8usize},
                "u128"|"i128"=>quote!{16usize},"Pubkey"=>quote!{32usize},
                "String"=>{let length=space_length(&segment.ident,lengths)?;quote!{(4usize + #length)}}
                "Vec"=>{let item=first.ok_or_else(||syn::Error::new_spanned(ty,"Invalid argument in Vec"))?;let length=space_length(&segment.ident,lengths)?;let size=space_type(item,lengths)?;quote!{(4usize + #size * #length)}}
                "Option"=>{let item=first.ok_or_else(||syn::Error::new_spanned(ty,"Invalid argument in Option"))?;let size=space_type(item,lengths)?;quote!{(1usize + #size)}}
                _=>quote!{<#ty as anchor_lang::Space>::INIT_SPACE},
            })
        }
        _=>Err(syn::Error::new_spanned(ty,"Tare does not implement this InitSpace type")),
    }
}
fn space_length(at:&Ident,lengths:&mut Option<std::collections::VecDeque<Tokens>>)->syn::Result<Tokens> {
    let lengths=lengths.as_mut().ok_or_else(||syn::Error::new_spanned(at,"Expected max_len attribute."))?;
    lengths.pop_front().ok_or_else(||syn::Error::new_spanned(at,"The number of lengths are invalid."))
}
fn parse_discriminator(args: TokenStream, namespace: &str, name: &Ident) -> syn::Result<Tokens> {
    if args.is_empty() {
        return Ok(discriminator(namespace, &name.to_string()));
    }
    use syn::parse::Parser;
    (|input: syn::parse::ParseStream| {
        let key: Ident = input.parse()?;
        if key != "discriminator" {
            return Err(syn::Error::new_spanned(
                key,
                "Tare does not implement this discriminator option",
            ));
        }
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        if !input.is_empty() {
            return Err(input.error("Unexpected discriminator argument"));
        }
        Ok(match value {
            Expr::Lit(ref lit) if matches!(lit.lit, syn::Lit::Int(_)) => quote! {&[#lit]},
            Expr::Array(ref array) => quote! {&#array},
            other => quote! {#other},
        })
    })
    .parse(args)
}
#[proc_macro_attribute]
pub fn event(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    let name = &item.ident;
    let disc = match parse_discriminator(args, "event", name) {
        Ok(d) => d,
        Err(e) => return e.to_compile_error().into(),
    };
    let (impl_g, ty_g, where_g) = item.generics.split_for_impl();
    quote!{
        #[derive(anchor_lang::AnchorSerialize,anchor_lang::AnchorDeserialize)] #item
        impl #impl_g anchor_lang::Discriminator for #name #ty_g #where_g {const DISCRIMINATOR:&'static[u8]=#disc;}
        impl #impl_g anchor_lang::Event for #name #ty_g #where_g {
            fn data(&self)->Vec<u8> {let mut bytes=Vec::with_capacity(256);bytes.extend_from_slice(<Self as anchor_lang::Discriminator>::DISCRIMINATOR);anchor_lang::AnchorSerialize::serialize(self,&mut bytes).unwrap();bytes}
        }
    }.into()
}
#[proc_macro_attribute]
pub fn event_cpi(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as ItemStruct);
    let life = if let Some(l) = item.generics.lifetimes().next() {
        l.lifetime.clone()
    } else {
        let l: syn::Lifetime = syn::parse_quote!('info);
        item.generics.params.insert(0, syn::parse_quote!('info));
        l
    };
    let syn::Fields::Named(fields) = &mut item.fields else {
        return fail(item, "event_cpi tuple accounts").into();
    };
    use syn::parse::Parser;
    let authority=if TARE_ANCHOR_MAJOR>=1 {
        quote!{#[account(address=crate::EVENT_AUTHORITY_AND_BUMP.0)] pub event_authority:anchor_lang::UncheckedAccount<#life>}
    }else{
        quote!{#[account(seeds=[b"__event_authority"],bump)] pub event_authority:anchor_lang::AccountInfo<#life>}
    };
    let program=if TARE_ANCHOR_MAJOR>=1 {quote!{pub program:anchor_lang::UncheckedAccount<#life>}}else{quote!{pub program:anchor_lang::AccountInfo<#life>}};
    fields.named.push(syn::Field::parse_named.parse2(authority).unwrap());
    fields.named.push(syn::Field::parse_named.parse2(program).unwrap());
    quote! {#item}.into()
}
#[proc_macro]
pub fn emit_cpi(input: TokenStream) -> TokenStream {
    let event = parse_macro_input!(input as Expr);
    let bump=if TARE_ANCHOR_MAJOR>=1 {quote!{crate::EVENT_AUTHORITY_AND_BUMP.1}}else{quote!{ctx.bumps.event_authority}};
    quote! {{
        let authority=anchor_lang::ToAccountInfo::__tare_info(&ctx.accounts.event_authority);
        let bump=#bump;
        let data=anchor_lang::Event::data(&#event);
        anchor_lang::event::emit_cpi(&crate::ID,authority,bump,data)?;
    }}
    .into()
}
#[proc_macro_attribute]
pub fn access_control(args: TokenStream, input: TokenStream) -> TokenStream {
    let guards = parse_macro_input!(args with syn::punctuated::Punctuated::<Expr,Token![,]>::parse_terminated);
    let guards: Vec<_> = guards.iter().collect();
    let mut item = parse_macro_input!(input as syn::ItemFn);
    let original = &item.block.stmts;
    item.block = Box::new(syn::parse_quote!({#(#guards?;)*#(#original)*}));
    quote! {#item}.into()
}
#[proc_macro_attribute]
pub fn constant(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}
#[proc_macro_attribute]
pub fn zero_copy(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as ItemStruct);
    let is_unsafe = if args.is_empty() {
        false
    } else if args.to_string() == "unsafe" {
        true
    } else {
        return fail(args.to_string(), "zero_copy argument").into();
    };
    let repr = if item.attrs.iter().any(|a| a.path.is_ident("repr")) {
        quote! {}
    } else if is_unsafe {
        quote! {#[repr(Rust,packed)]}
    } else {
        quote! {#[repr(C)]}
    };
    let mut methods = Vec::new();
    for field in &mut item.fields {
        if let Some(attr) = field.attrs.iter().find(|a| a.path.is_ident("accessor")) {
            let ty = match attr.parse_args::<Type>() {
                Ok(ty) => ty,
                Err(e) => return e.to_compile_error().into(),
            };
            let id = field.ident.as_ref().unwrap();
            let get = format_ident!("get_{}", id);
            let set = format_ident!("set_{}", id);
            methods.push(quote!{pub fn #get(&self)->#ty{anchor_lang::__private::ZeroCopyAccessor::get(&self.#id)}pub fn #set(&mut self,input:&#ty){self.#id=anchor_lang::__private::ZeroCopyAccessor::set(input);}});
            field.attrs.retain(|a| !a.path.is_ident("accessor"));
        }
    }
    let derive_text = item
        .attrs
        .iter()
        .map(|a| a.tokens.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let pod = if is_unsafe || derive_text.contains("bytemuck :: Pod") {
        quote! {}
    } else {
        quote! {#[derive(::bytemuck::Pod)]}
    };
    let zeroable = if is_unsafe || derive_text.contains("bytemuck :: Zeroable") {
        quote! {}
    } else {
        quote! {#[derive(::bytemuck::Zeroable)]}
    };
    let name = &item.ident;
    let (ig, tg, wg) = item.generics.split_for_impl();
    quote! {#[derive(Copy,Clone)] #repr #pod #zeroable #item impl #ig #name #tg #wg {#(#methods)*}}
        .into()
}
fn account_options(
    args: TokenStream,
) -> syn::Result<(Option<bool>, Option<String>, Option<Tokens>)> {
    use syn::parse::Parser;
    (|input: syn::parse::ParseStream| {
        let mut zero = None;
        let mut namespace = None;
        let mut disc = None;
        while !input.is_empty() {
            if input.peek(syn::LitStr) {
                namespace = Some(input.parse::<syn::LitStr>()?.value());
            } else {
                let id: Ident = input.parse()?;
                if id == "zero_copy" {
                    let mut is_unsafe = false;
                    if input.peek(syn::token::Paren) {
                        let content;
                        syn::parenthesized!(content in input);
                        content.parse::<Token![unsafe]>()?;
                        is_unsafe = true;
                    }
                    zero = Some(is_unsafe);
                } else if id == "discriminator" {
                    input.parse::<Token![=]>()?;
                    let value: Expr = input.parse()?;
                    disc = Some(match value {
                        Expr::Lit(ref lit) if matches!(lit.lit, syn::Lit::Int(_)) => {
                            quote! {&[#lit]}
                        }
                        Expr::Array(ref array) => quote! {&#array},
                        other => quote! {#other},
                    });
                } else {
                    return Err(syn::Error::new_spanned(
                        id,
                        "Tare does not implement account argument",
                    ));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok((zero, namespace, disc))
    })
    .parse(args)
}

struct NestedMeta<'a> {
    path: syn::Path,
    input: syn::parse::ParseStream<'a>,
}
impl<'a> NestedMeta<'a> {
    fn value(&self) -> syn::Result<syn::parse::ParseStream<'a>> {
        self.input.parse::<Token![=]>()?;
        Ok(self.input)
    }
    fn error(&self, message: impl std::fmt::Display) -> syn::Error {
        syn::Error::new_spanned(&self.path, message)
    }
}
trait AttributeMeta {
    fn parse_nested_meta(
        &self,
        f: impl FnMut(NestedMeta<'_>) -> syn::Result<()>,
    ) -> syn::Result<()>;
}
impl AttributeMeta for syn::Attribute {
    fn parse_nested_meta(
        &self,
        mut f: impl FnMut(NestedMeta<'_>) -> syn::Result<()>,
    ) -> syn::Result<()> {
        use syn::ext::IdentExt;
        self.parse_args_with(|input: syn::parse::ParseStream| {
            while !input.is_empty() {
                let mut segments = syn::punctuated::Punctuated::new();
                segments.push_value(syn::PathSegment::from(input.call(Ident::parse_any)?));
                while input.peek(Token![::]) {
                    segments.push_punct(input.parse()?);
                    segments.push_value(syn::PathSegment::from(input.call(Ident::parse_any)?));
                }
                f(NestedMeta {
                    path: syn::Path {
                        leading_colon: None,
                        segments,
                    },
                    input,
                })?;
                if input.is_empty() {
                    break;
                }
                input.parse::<Token![,]>()?;
            }
            Ok(())
        })
    }
}

fn account_inner(ty: &Type) -> syn::Result<(bool, bool, &Type)> {
    let mut inner = ty;
    let mut optional = false;
    let mut boxed = false;
    if let Type::Path(p) = inner {
        if p.path.segments.last().unwrap().ident == "Option" {
            optional = true;
            inner = type_argument(inner)?;
        }
    }
    if let Type::Path(p) = inner {
        if p.path.segments.last().unwrap().ident == "Box" {
            boxed = true;
            inner = type_argument(inner)?;
        }
    }
    Ok((optional, boxed, inner))
}
fn type_argument(ty: &Type) -> syn::Result<&Type> {
    if let Type::Path(p) = ty {
        if let PathArguments::AngleBracketed(args) = &p.path.segments.last().unwrap().arguments {
            if let Some(GenericArgument::Type(ty)) = args.args.last() {
                return Ok(ty);
            }
        }
    }
    Err(syn::Error::new_spanned(
        ty,
        "Tare requires a concrete account wrapper type",
    ))
}

fn optional_check(expr: &Expr, fields: &[&Ident]) -> Tokens {
    if let Expr::Path(path) = expr {
        if let Some(id) = path.path.get_ident() {
            if fields.iter().any(|f| **f == *id) {
                let name = id.to_string();
                return quote! {let #id=if let Some(ref value)=#id{value}else{return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintAccountIsNone).__tare_with_account_name_static(#name));};};
            }
        }
    }
    quote! {}
}

fn parse_public_key(input:TokenStream)->syn::Result<Tokens>{
    let expr=syn::parse::<Expr>(input)?;
    if let Expr::Lit(lit)=&expr {if let syn::Lit::Str(text)=&lit.lit {
        let bytes=bs58::decode(text.value()).into_vec().map_err(|e|syn::Error::new_spanned(text,e))?;
        if bytes.len()!=32{return Err(syn::Error::new_spanned(text,"pubkey array is not 32 bytes long"));}
        return Ok(quote!{anchor_lang::Pubkey::new_from_array([#(#bytes),*])});
    }}
    Ok(quote!{#expr})
}
#[proc_macro]
pub fn declare_id(input:TokenStream)->TokenStream {
    let id=match parse_public_key(input){Ok(id)=>id,Err(e)=>return e.to_compile_error().into()};
    let event_authority=if TARE_ANCHOR_MAJOR>=1 && cfg!(feature="event-cpi") {
        quote!{pub const EVENT_AUTHORITY_AND_BUMP:(anchor_lang::Pubkey,u8)={
            let (address,bump)=anchor_lang::derive_program_address(&[b"__event_authority"],&ID_CONST.to_bytes());
            (anchor_lang::Pubkey::new_from_array(address),bump)
        };}
    }else{quote!{}};
    quote!{
        pub static ID:anchor_lang::Pubkey=#id;
        pub const ID_CONST:anchor_lang::Pubkey=#id;
        pub fn check_id(id:&anchor_lang::Pubkey)->bool{id==&ID}
        pub fn id()->anchor_lang::Pubkey{ID}
        pub const fn id_const()->anchor_lang::Pubkey{ID_CONST}
        #event_authority
    }.into()
}
#[proc_macro]
pub fn pubkey(input:TokenStream)->TokenStream {
    match parse_public_key(input){Ok(id)=>id.into(),Err(e)=>e.to_compile_error().into()}
}


fn spl_init(
    c: &Constraints, id: &Ident, ty: &Type, wrapper: &Tokens, name: &str,
    seeds: &Tokens, optional: &[&Ident],
) -> syn::Result<Tokens> {
    let payer = c.payer.as_ref().ok_or_else(|| syn::Error::new(id.span(), "init requires payer"))?;
    let associated = c.associated_mint.is_some() || c.associated_authority.is_some();
    let mint_init = c.mint_decimals.is_some() || c.mint_authority.is_some() || c.mint_freeze_authority.is_some() || c.mint_program.is_some();
    let token_init = c.token_mint.is_some() || c.token_authority.is_some() || c.token_program.is_some();
    if usize::from(associated)+usize::from(mint_init)+usize::from(token_init)!=1 {
        return Err(syn::Error::new(id.span(), "conflicting SPL initialization constraints"));
    }
    if associated && c.seeds.is_some() { return Err(syn::Error::new(id.span(), "seeds cannot be provided with associated_token init")); }
    let (mint, authority, program) = if associated {
        (c.associated_mint.as_ref(), c.associated_authority.as_ref(), c.associated_program.as_ref())
    } else if mint_init {
        (None, c.mint_authority.as_ref(), c.mint_program.as_ref())
    } else {
        (c.token_mint.as_ref(), c.token_authority.as_ref(), c.token_program.as_ref())
    };
    let authority = authority.ok_or_else(|| syn::Error::new(id.span(), "SPL init requires authority"))?;
    let default_program: Expr = syn::parse_quote!(token_program);
    let program = program.unwrap_or(&default_program);
    let system: Expr = syn::parse_quote!(system_program);
    let associated_program: Expr = syn::parse_quote!(associated_token_program);
    let rent: Expr = syn::parse_quote!(rent);
    let mut required = vec![&system, program];
    if associated { required.push(&associated_program); }
    required.push(&rent);
    required.push(authority);
    if let Some(mint) = mint { required.push(mint); }
    if let Some(freeze) = &c.mint_freeze_authority { required.push(freeze); }
    let mut seen = std::collections::BTreeSet::new();
    seen.insert(id.to_string());
    let checks = required.into_iter().filter_map(|expr| {
        seen.insert(expr.to_token_stream().to_string()).then(|| optional_check(expr,optional))
    }).collect::<Vec<_>>();
    let payer_check = if seen.contains(&payer.to_token_stream().to_string()) {quote!{}} else {optional_check(payer,optional)};
    let if_needed = c.if_needed;
    let make_account = |space: Tokens| quote! {
        if #id.lamports()!=0 && #payer.key()==#id.key(){return Err(anchor_lang::error!(anchor_lang::ErrorCode::TryingToInitPayerAsProgramAccount).with_pubkeys((#payer.key(),#id.key())));}
        let __space:usize=#space;
        anchor_lang::cpi::create_account(#payer.__tare_info(),#id,system_program.__tare_info(),&#program.key(),__space,&__anchor_rent,#seeds)?;
    };
    let (create, validation) = if associated {
        let mint = mint.ok_or_else(||syn::Error::new(id.span(),"associated_token init requires mint"))?;
        (
            quote! {
                ({#[inline(never)] || -> anchor_lang::Result<()> {
                    let __tare_cpi_program=associated_token_program.to_account_info();
                    let __cpi_accounts=::anchor_spl::associated_token::Create{
                        payer:#payer.to_account_info(),associated_token:#id.to_account_info(),authority:#authority.to_account_info(),
                        mint:#mint.to_account_info(),system_program:system_program.to_account_info(),token_program:#program.to_account_info(),
                    };
                    ::anchor_spl::associated_token::create(anchor_lang::CpiContext::new(__tare_cpi_program,__cpi_accounts))?;
                    Ok(())
                }})()?;
            },
            quote! {
                if __value.mint!=#mint.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintTokenMint).__tare_with_account_name_static(#name).with_pubkeys((__value.mint,#mint.key())));}
                if __value.owner!=#authority.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintTokenOwner).__tare_with_account_name_static(#name).with_pubkeys((__value.owner,#authority.key())));}
                if __owner_program!=&#program.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintAssociatedTokenTokenProgram).__tare_with_account_name_static(#name).with_pubkeys((*__owner_program,#program.key())));}
                if __value.key()!=::anchor_spl::associated_token::get_associated_token_address_with_program_id(&#authority.key(),&#mint.key(),&#program.key()){
                    return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::AccountNotAssociatedTokenAccount).__tare_with_account_name_static(#name));
                }
            },
        )
    } else if mint_init {
        let decimals=c.mint_decimals.as_ref().ok_or_else(||syn::Error::new(id.span(),"mint init requires decimals"))?;
        let freeze=if let Some(freeze)=&c.mint_freeze_authority {quote!{Some(&#freeze.key())}}else{quote!{Option::<&anchor_lang::Pubkey>::None}};
        let account=make_account(quote!{::anchor_spl::token::Mint::LEN});
        (
            quote! {
                #account
                let __tare_cpi_program=#program.to_account_info();
                let __cpi_accounts=::anchor_spl::token_interface::InitializeMint2{mint:#id.to_account_info()};
                ::anchor_spl::token_interface::initialize_mint2(anchor_lang::CpiContext::new(__tare_cpi_program,__cpi_accounts),#decimals,&#authority.key(),#freeze)?;
            },
            quote! {
                if __value.mint_authority!=anchor_lang::solana_program::program_option::COption::Some(#authority.key()){
                    return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintMintMintAuthority).__tare_with_account_name_static(#name));
                }
                if __value.freeze_authority.as_ref().map(|actual|#freeze.as_ref().map(|expected|actual!=*expected).unwrap_or(true)).unwrap_or(#freeze.is_some()){
                    return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintMintFreezeAuthority).__tare_with_account_name_static(#name));
                }
                if __value.decimals!=#decimals{return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintMintDecimals).__tare_with_account_name_static(#name).with_values((__value.decimals,#decimals)));}
                if __owner_program!=&#program.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintMintTokenProgram).__tare_with_account_name_static(#name).with_pubkeys((*__owner_program,#program.key())));}
            },
        )
    } else {
        let mint=mint.ok_or_else(||syn::Error::new(id.span(),"token init requires mint"))?;
        let space=quote!{{
            let __mint_info=#mint.to_account_info();
            if *__mint_info.owner==<::anchor_spl::token_2022::Token2022 as anchor_lang::Id>::id(){
                use ::anchor_spl::token_2022::spl_token_2022::extension::{BaseStateWithExtensions,ExtensionType,StateWithExtensions};
                use ::anchor_spl::token_2022::spl_token_2022::state::{Account,Mint};
                let __mint_data=__mint_info.try_borrow_data()?;
                let __mint=StateWithExtensions::<Mint>::unpack(&__mint_data)?;
                let __extensions=__mint.get_extension_types()?;
                let __required=ExtensionType::get_required_init_account_extensions(&__extensions);
                ExtensionType::try_calculate_account_len::<Account>(&__required)?
            }else{::anchor_spl::token::TokenAccount::LEN}
        }};
        let account=make_account(space);
        (
            quote! {
                #account
                let __tare_cpi_program=#program.to_account_info();
                let __cpi_accounts=::anchor_spl::token_interface::InitializeAccount3{account:#id.to_account_info(),mint:#mint.to_account_info(),authority:#authority.to_account_info()};
                ::anchor_spl::token_interface::initialize_account3(anchor_lang::CpiContext::new(__tare_cpi_program,__cpi_accounts))?;
            },
            quote! {
                if __value.mint!=#mint.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintTokenMint).__tare_with_account_name_static(#name).with_pubkeys((__value.mint,#mint.key())));}
                if __value.owner!=#authority.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintTokenOwner).__tare_with_account_name_static(#name).with_pubkeys((__value.owner,#authority.key())));}
                if __owner_program!=&#program.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintTokenTokenProgram).__tare_with_account_name_static(#name).with_pubkeys((*__owner_program,#program.key())));}
            },
        )
    };
    Ok(quote! {
        #(#checks)*
        let __owner_program=#id.owner_ref();
        if !#if_needed || __owner_program==&anchor_lang::solana_program::system_program::ID {
            #payer_check
            #create
        }
        let __value:#ty=#wrapper::__try_from_unchecked(#id,false).map_err(|error|error.__tare_with_account_name_static(#name))?;
        if #if_needed {#validation}
        __tare_spl_output.write(__value);
        Ok(())
    })
}

fn associated_check(c:&Constraints,id:&Ident,name:&str,optional:&[&Ident])->syn::Result<Option<Tokens>> {
    if c.associated_mint.is_none() && c.associated_authority.is_none() && c.associated_program.is_none(){return Ok(None)}
    let mint=c.associated_mint.as_ref().ok_or_else(||syn::Error::new(id.span(),"associated_token constraint requires mint"))?;
    let authority=c.associated_authority.as_ref().ok_or_else(||syn::Error::new(id.span(),"associated_token constraint requires authority"))?;
    let authority_check=optional_check(authority,optional);let mint_check=optional_check(mint,optional);
    let (program_check,address)=if let Some(program)=&c.associated_program {
        let check=optional_check(program,optional);
        (quote!{#check if #id.__tare_info().owner_ref()!=&#program.key(){return Err(anchor_lang::ErrorCode::ConstraintAssociatedTokenTokenProgram.into());}},quote!{::anchor_spl::associated_token::get_associated_token_address_with_program_id(&__wallet,&#mint.key(),&#program.key())})
    }else{(quote!{},quote!{::anchor_spl::associated_token::get_associated_token_address(&__wallet,&#mint.key())})};
    Ok(Some(quote!{{
        #authority_check #mint_check #program_check
        let __wallet=#authority.key();
        if #id.owner!=__wallet{return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintTokenOwner).__tare_with_account_name_static(#name).with_pubkeys((#id.owner,__wallet)));}
        let __address=#address;
        if #id.key()!=__address{return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintAssociated).__tare_with_account_name_static(#name).with_pubkeys((#id.key(),__address)));}
    }}))
}

fn spl_checks(c:&Constraints,id:&Ident,optional:&[&Ident])->Vec<Tokens>{
    let mut out=Vec::new();
    let name=id.to_string();
    if let Some(authority)=&c.token_authority {
        let check=optional_check(authority,optional);
        out.push(if TARE_ANCHOR_MAJOR>=1 {
            quote!{{#check if #id.owner!=#authority.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintTokenOwner).__tare_with_account_name_static(#name).with_pubkeys((#id.owner,#authority.key())));}}}
        }else{quote!{{#check if #id.owner!=#authority.key(){return Err(anchor_lang::ErrorCode::ConstraintTokenOwner.into());}}}});
    }
    if let Some(mint)=&c.token_mint {
        let check=optional_check(mint,optional);
        out.push(if TARE_ANCHOR_MAJOR>=1 {
            quote!{{#check if #id.mint!=#mint.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintTokenMint).__tare_with_account_name_static(#name).with_pubkeys((#id.mint,#mint.key())));}}}
        }else{quote!{{#check if #id.mint!=#mint.key(){return Err(anchor_lang::ErrorCode::ConstraintTokenMint.into());}}}});
    }
    if let Some(program)=&c.token_program {
        let check=optional_check(program,optional);
        out.push(if TARE_ANCHOR_MAJOR>=1 {
            quote!{{#check if #id.__tare_info().owner_ref()!=&#program.key(){return Err(anchor_lang::Error::from(anchor_lang::ErrorCode::ConstraintTokenTokenProgram).__tare_with_account_name_static(#name).with_pubkeys((*#id.__tare_info().owner_ref(),#program.key())));}}}
        }else{quote!{{#check if #id.__tare_info().owner_ref()!=&#program.key(){return Err(anchor_lang::ErrorCode::ConstraintTokenTokenProgram.into());}}}});
    }
    if let Some(decimals)=&c.mint_decimals {out.push(quote!{if #id.decimals!=#decimals{return Err(anchor_lang::ErrorCode::ConstraintMintDecimals.into());}});}
    if let Some(authority)=&c.mint_authority {let check=optional_check(authority,optional);out.push(quote!{{#check if #id.mint_authority!=anchor_lang::solana_program::program_option::COption::Some(#authority.key()){return Err(anchor_lang::ErrorCode::ConstraintMintMintAuthority.into());}}});}
    if let Some(authority)=&c.mint_freeze_authority {let check=optional_check(authority,optional);out.push(quote!{{#check if #id.freeze_authority!=anchor_lang::solana_program::program_option::COption::Some(#authority.key()){return Err(anchor_lang::ErrorCode::ConstraintMintFreezeAuthority.into());}}});}
    if let Some(program)=&c.mint_program {let check=optional_check(program,optional);out.push(quote!{{#check if #id.__tare_info().owner_ref()!=&#program.key(){return Err(anchor_lang::ErrorCode::ConstraintMintTokenProgram.into());}}});}
    out
}


#[proc_macro]
pub fn declare_program(input:TokenStream)->TokenStream {
    declare_program_impl::expand(input.into()).unwrap_or_else(|error|error.to_compile_error()).into()
}
#[proc_macro_attribute]
pub fn instruction(_args:TokenStream,input:TokenStream)->TokenStream { input }
#[proc_macro_attribute]
pub fn interface(_args:TokenStream,input:TokenStream)->TokenStream { input }

fn instruction_discriminator(fun:&syn::ItemFn)->syn::Result<Option<Tokens>> {
    let mut interface=None;
    let mut custom=None;
    for attr in &fun.attrs {
        if attr.path.is_ident("interface") {
            if interface.is_some(){return Err(syn::Error::new_spanned(attr,"An instruction can only implement one interface instruction"));}
            let path=attr.parse_args::<syn::Path>()?;
            let name=path.segments.iter().map(|s|s.ident.to_string()).collect::<Vec<_>>().join("::");
            let bytes=match name.as_str(){
                "spl_transfer_hook_interface::initialize_extra_account_meta_list"=>[43u8,34,13,49,167,88,235,235],
                "spl_transfer_hook_interface::execute"=>[105u8,37,101,197,75,251,102,26],
                _=>return Err(syn::Error::new_spanned(path,format!("Tare does not implement interface {name}"))),
            };
            interface=Some(quote!{&[#(#bytes),*]});
        }else if attr.path.is_ident("instruction") {
            let value=attr.parse_args_with(|input:syn::parse::ParseStream| {
                let key:Ident=input.parse()?;
                if key!="discriminator"{return Err(syn::Error::new_spanned(key,"Tare does not implement this instruction override"));}
                input.parse::<Token![=]>()?;
                let value:Expr=input.parse()?;
                if input.peek(Token![,]){input.parse::<Token![,]>()?;}
                if !input.is_empty(){return Err(input.error("Unexpected instruction override"));}
                Ok(value)
            })?;
            custom=Some(match value {
                Expr::Lit(ref lit) if matches!(lit.lit,syn::Lit::Int(_))=>quote!{&[#lit]},
                Expr::Array(ref array)=>quote!{&#array},
                other=>quote!{#other},
            });
        }
    }
    Ok(custom.or(interface))
}

// The Context's slice is materialized only if its contents can be observed.
// Unknown macros/attributes may introduce caller-context references, so they
// are conservatively treated as escapes even when their tokens omit `ctx`.
fn context_may_read_remaining(fun:&syn::ItemFn,context:&Ident)->bool {
    for attr in &fun.attrs {
        if !matches!(attr.path.get_ident().map(|i|i.to_string()).as_deref(),Some("doc"|"allow"|"warn"|"deny"|"forbid"|"cfg"|"inline"|"cold")) {return true;}
    }
    fn scan(tokens:Tokens,context:&Ident)->bool {
        use proc_macro2::TokenTree;
        let tokens=tokens.into_iter().collect::<Vec<_>>();
        for (index,token) in tokens.iter().enumerate() {
            if matches!(token,TokenTree::Punct(p) if p.as_char()=='!') && matches!(tokens.get(index+1),Some(TokenTree::Group(_))) {return true;}
            if let TokenTree::Ident(id)=token {
                if id=="remaining_accounts" {return true;}
                if id==context {
                    let field=match (tokens.get(index+1),tokens.get(index+2)) {
                        (Some(TokenTree::Punct(dot)),Some(TokenTree::Ident(field))) if dot.as_char()=='.'=>Some(field.to_string()),_=>None,
                    };
                    if !matches!(field.as_deref(),Some("accounts"|"bumps"|"program_id")){return true;}
                }
            }
            if let TokenTree::Group(group)=token {if scan(group.stream(),context){return true;}}
        }
        false
    }
    scan(fun.block.to_token_stream(),context)
}

fn is_leaf_account(ty:&Type)->bool {
    matches!(ty,Type::Path(p) if matches!(p.path.segments.last().unwrap().ident.to_string().as_str(),"Account"|"Signer"|"Program"|"SystemAccount"|"UncheckedAccount"|"AccountInfo"|"Interface"|"InterfaceAccount"|"AccountLoader"|"Sysvar"))
}

//! Conservative source detector for observable account representation.
use anyhow::{Context, Result};
use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use syn::{
    visit::{self, Visit},
    GenericArgument, Item, PathArguments, Type,
};

#[derive(Clone)]
enum Definition {
    Fields(Vec<Type>),
    GenericFields(Vec<String>, Vec<Type>),
    Alias(Type),
    ProvenIndependent,
}
#[derive(Clone)]
struct Types {
    definitions: BTreeMap<String, Option<Definition>>,
    observations: BTreeMap<String, String>,
    external_glob: bool,
    payload_imports: BTreeMap<String,Type>,
    closed_generic: BTreeSet<String>,
}
impl Types {
    fn collect(files: &[(PathBuf, syn::File)]) -> Self {
        fn collect_items(items: &[Item], definitions: &mut BTreeMap<String, Option<Definition>>) {
            for item in items {
                let named = match item {
                    Item::Struct(s) => Some((
                        s.ident.to_string(),
                        if s.generics.type_params().next().is_some() {
                            Definition::GenericFields(s.generics.type_params().map(|p|p.ident.to_string()).collect(), s.fields.iter().map(|f|f.ty.clone()).collect())
                        } else { Definition::Fields(s.fields.iter().map(|f| f.ty.clone()).collect()) },
                    )),
                    Item::Enum(e) => Some((
                        e.ident.to_string(),
                        Definition::Fields(
                            e.variants
                                .iter()
                                .flat_map(|v| v.fields.iter().map(|f| f.ty.clone()))
                                .collect(),
                        ),
                    )),
                    Item::Union(u) => Some((
                        u.ident.to_string(),
                        Definition::Fields(u.fields.named.iter().map(|f| f.ty.clone()).collect()),
                    )),
                    Item::Type(t) => {
                        Some((t.ident.to_string(), Definition::Alias((*t.ty).clone())))
                    }
                    _ => None,
                };
                if let Some((name, definition)) = named {
                    if definitions.contains_key(&name) {
                        definitions.insert(name, None);
                    } else {
                        definitions.insert(name, Some(definition));
                    }
                }
                if let Item::Mod(m) = item {
                    definitions.entry(m.ident.to_string()).or_insert(None);
                    if let Some((_, content)) = &m.content {
                        collect_items(content, definitions)
                    }
                }
            }
        }
        let mut definitions = BTreeMap::new();
        for (_, file) in files {
            collect_items(&file.items, &mut definitions)
        }
        struct Integers<'a>(&'a mut BTreeMap<String, Option<Definition>>);
        impl<'ast> Visit<'ast> for Integers<'_> {
            fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
                if node.mac.path.segments.last().is_some_and(|s|s.ident=="construct_uint") {
                    struct IntegerDecl { name:syn::Ident, limbs:syn::LitInt }
                    impl syn::parse::Parse for IntegerDecl {
                        fn parse(input:syn::parse::ParseStream)->syn::Result<Self> {
                            let _visibility: syn::Visibility=input.parse()?;
                            input.parse::<syn::Token![struct]>()?;
                            let name=input.parse()?;let content;syn::parenthesized!(content in input);let limbs=content.parse()?;
                            input.parse::<syn::Token![;]>()?;
                            if !input.is_empty(){return Err(input.error("one integer declaration expected"))}
                            Ok(Self{name,limbs})
                        }
                    }
                    if let Ok(decl)=syn::parse2::<IntegerDecl>(node.mac.tokens.clone()) {
                        if let Ok(limbs)=decl.limbs.base10_parse::<usize>() {
                            if limbs>0 {
                                let ty:Type=syn::parse_quote!([u64; #limbs]);
                                self.0.entry(decl.name.to_string()).or_insert(Some(Definition::Fields(vec![ty])));
                            }
                        }
                    }
                }
                visit::visit_item_macro(self,node);
            }
        }
        // Only the canonical uint macro has this representation. A local macro
        // with the same name remains unknown.
        let local_uint_macro=files.iter().any(|(_,f)|f.items.iter().any(|i|matches!(i,Item::Macro(m) if m.ident.as_ref().is_some_and(|id|id=="construct_uint"))));
        let canonical_uint=!local_uint_macro && files.iter().any(|(_,f)|f.items.iter().any(|i|matches!(i,Item::Use(u) if u.to_token_stream().to_string().replace(' ',"")=="useuint::construct_uint;")));
        if canonical_uint { for (_,file) in files {Integers(&mut definitions).visit_file(file)} }
        struct BlockDefinitions<'a>(&'a mut BTreeMap<String, Option<Definition>>);
        impl<'ast> Visit<'ast> for BlockDefinitions<'_> {
            fn visit_block(&mut self, block: &'ast syn::Block) {
                for statement in &block.stmts {
                    if let syn::Stmt::Item(item) = statement {
                        collect_items(std::slice::from_ref(item), self.0);
                    }
                }
                visit::visit_block(self, block);
            }
        }
        for (_, file) in files {
            BlockDefinitions(&mut definitions).visit_file(file);
        }
        fn aliases(tree: &syn::UseTree, out: &mut BTreeMap<String, String>) {
            match tree {
                syn::UseTree::Path(p) => aliases(&p.tree, out),
                syn::UseTree::Group(g) => {
                    for t in &g.items {
                        aliases(t, out)
                    }
                }
                syn::UseTree::Rename(r) => {
                    let name = r.ident.to_string();
                    if [
                        "size_of",
                        "align_of",
                        "type_name",
                        "TypeId",
                        "transmute",
                        "transmute_copy",
                        "size_of_val",
                        "align_of_val",
                        "addr_of",
                        "addr_of_mut",
                        "Any",
                        "Rc",
                        "RefCell",
                    ]
                    .contains(&name.as_str())
                    {
                        out.insert(r.rename.to_string(), name);
                    }
                }
                _ => {}
            }
        }
        fn type_imports(
            tree: &syn::UseTree,
            prefix: &mut Vec<String>,
            names: &mut BTreeSet<String>,
            external_glob: &mut bool,
        ) {
            match tree {
                syn::UseTree::Path(p) => {
                    prefix.push(p.ident.to_string());
                    type_imports(&p.tree, prefix, names, external_glob);
                    prefix.pop();
                }
                syn::UseTree::Group(g) => {
                    for t in &g.items {
                        type_imports(t, prefix, names, external_glob)
                    }
                }
                syn::UseTree::Name(n) => {
                    if !prefix
                        .first()
                        .is_some_and(|p| p == "crate" || p == "self" || p == "super")
                    {
                        names.insert(n.ident.to_string());
                    }
                }
                syn::UseTree::Rename(n) => {
                    // Renames can shadow a built-in payload name even through a local module.
                    // Resolving the target needs full Rust name resolution, so retain SDK.
                    names.insert(n.rename.to_string());
                }
                syn::UseTree::Glob(_) => {
                    if !prefix.first().is_some_and(|p| {
                        ["crate", "self", "super", "anchor_lang", "std", "core", "pinocchio"].contains(&p.as_str())
                    }) {
                        *external_glob = true
                    }
                }
            }
        }
        fn all_type_imports(items: &[Item], names: &mut BTreeSet<String>, glob: &mut bool) {
            for item in items {
                match item {
                    Item::Use(u) => type_imports(&u.tree, &mut vec![], names, glob),
                    Item::Mod(m) => {
                        if let Some((_, content)) = &m.content {
                            all_type_imports(content, names, glob)
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut imported = BTreeSet::new();
        let mut external_glob = false;
        for (_, file) in files {
            all_type_imports(&file.items, &mut imported, &mut external_glob)
        }
        for name in imported {
            definitions.insert(name, None);
        }
        struct GenericNames(BTreeSet<String>);
        impl<'ast> Visit<'ast> for GenericNames {
            fn visit_type_param(&mut self, p: &'ast syn::TypeParam) {
                self.0.insert(p.ident.to_string());
            }
        }
        let mut generic_names = GenericNames(BTreeSet::new());
        for (_, file) in files {
            generic_names.visit_file(file)
        }
        for name in generic_names.0 {
            definitions.insert(name, None);
        }
        struct Uses {
            observations: BTreeMap<String, String>,
            names: BTreeSet<String>,
            external_glob: bool,
        }
        impl<'ast> Visit<'ast> for Uses {
            fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
                aliases(&u.tree, &mut self.observations);
                type_imports(
                    &u.tree,
                    &mut vec![],
                    &mut self.names,
                    &mut self.external_glob,
                );
            }
        }
        let mut uses = Uses {
            observations: BTreeMap::new(),
            names: BTreeSet::new(),
            external_glob,
        };
        for (_, file) in files {
            uses.visit_file(file)
        }
        for name in uses.names {
            definitions.insert(name, None);
        }
        Self {
            definitions,
            observations: uses.observations,
            external_glob: uses.external_glob,
            payload_imports: BTreeMap::new(),
            closed_generic:BTreeSet::new(),
        }
    }
    fn independent(&self, ty: &Type, stack: &mut BTreeSet<String>) -> bool {
        match ty {
            Type::Array(a) => self.independent(&a.elem, stack),
            Type::Slice(a) => self.independent(&a.elem, stack),
            Type::Reference(a) => self.independent(&a.elem, stack),
            Type::Ptr(a) => self.independent(&a.elem, stack),
            Type::Paren(a) => self.independent(&a.elem, stack),
            Type::Group(a) => self.independent(&a.elem, stack),
            Type::Tuple(t) => t.elems.iter().all(|t| self.independent(t, stack)),
            Type::Never(_) => true,
            Type::Path(p) if p.qself.is_none() => {
                let Some(last) = p.path.segments.last() else {
                    return false;
                };
                let name = last.ident.to_string();
                if let Some(payload)=self.payload_imports.get(&name) {
                    if p.path.segments.len()==1 {return self.independent(payload,stack)}
                }
                if [
                    "AccountInfo",
                    "AccountData",
                    "AccountLamports",
                    "DataRef",
                    "LamportsRef",
                    "Account",
                    "AccountLoader",
                    "InterfaceAccount",
                    "Signer",
                    "Program",
                    "Interface",
                    "SystemAccount",
                    "UncheckedAccount",
                    "Sysvar",
                    "Context",
                    "CpiContext",
                    "Any",
                    "Ref",
                    "RefMut",
                ]
                .contains(&name.as_str())
                {
                    return false;
                }
                let arguments = match &last.arguments {
                    PathArguments::None => vec![],
                    PathArguments::AngleBracketed(a) => {
                        let mut out = vec![];
                        for arg in &a.args {
                            match arg {
                                GenericArgument::Type(t) => out.push(t),
                                GenericArgument::Lifetime(_) | GenericArgument::Const(_) => {}
                                _ => return false,
                            }
                        }
                        out
                    }
                    _ => return false,
                };
                if !arguments.iter().all(|t| self.independent(t, stack)) {
                    return false;
                }
                let known_path = p.path.segments.len() == 1
                    || p.path.segments.first().is_some_and(|s| {
                        !self.definitions.contains_key(&s.ident.to_string())
                            && ["std", "core", "alloc", "solana_program", "anchor_lang"]
                                .contains(&s.ident.to_string().as_str())
                    });
                if known_path
                    && !self.definitions.contains_key(&name)
                    && [
                        "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64",
                        "i128", "isize", "f32", "f64", "bool", "char", "str", "String", "Pubkey",
                        "Duration",
                    ]
                    .contains(&name.as_str())
                {
                    return true;
                }
                if known_path
                    && !self.definitions.contains_key(&name)
                    && [
                        "Vec",
                        "Option",
                        "Result",
                        "Box",
                        "PhantomData",
                        "MaybeUninit",
                        "ManuallyDrop",
                        "Cell",
                        "RefCell",
                        "Rc",
                        "Arc",
                        "BTreeMap",
                        "BTreeSet",
                        "HashMap",
                        "HashSet",
                    ]
                    .contains(&name.as_str())
                {
                    return !arguments.is_empty();
                }
                // Local lookup accepts only the unqualified name or Self-module form.
                // Other module paths require name resolution and remain conservative.
                if p.path.segments.len() > 1
                    && !(p.path.segments.len() == 2 && p.path.segments[0].ident == "self")
                {
                    return false;
                }
                // Ambiguous local names, recursive or generic types remain conservative.
                if self.external_glob || (!arguments.is_empty() && !matches!(self.definitions.get(&name), Some(Some(Definition::GenericFields(..))))) || !stack.insert(name.clone()) {
                    return false;
                }
                let answer = match self.definitions.get(&name).and_then(Option::as_ref) {
                    Some(Definition::Fields(fields)) => {
                        fields.iter().all(|t| self.independent(t, stack))
                    }
                    Some(Definition::Alias(ty)) => self.independent(ty, stack),
                    Some(Definition::ProvenIndependent) => true,
                    Some(Definition::GenericFields(names, fields)) if names.len()==arguments.len() => {
                        let bindings:BTreeMap<String,Type>=names.iter().cloned().zip(arguments.iter().map(|t|(*t).clone())).collect();
                        fields.iter().all(|ty|self.independent(&substitute(ty,&bindings),stack))
                    }
                    _ => false,
                };
                stack.remove(&name);
                answer
            }
            _ => false,
        }
    }
}
#[derive(Clone)]
struct LocalDefinition { module:Vec<String>, params:Vec<String>, fields:Vec<Type> }
#[derive(Default)]
struct ModuleNames { imports:BTreeMap<String,Vec<String>>, globs:Vec<Vec<String>> }
#[derive(Default)]
struct Qualified {
    definitions:BTreeMap<String,LocalDefinition>,
    modules:BTreeMap<String,ModuleNames>,
    block_names:BTreeSet<String>,
    local_modules:BTreeSet<String>,
}
fn name_key(path:&[String])->String {path.join("::")}
fn imported_path(module:&[String],path:&[String])->Vec<String> {
    let mut out=module.to_vec();let mut cursor=0;
    if path.first().is_some_and(|p|p=="crate"){out.clear();cursor=1;}
    else if path.first().is_some_and(|p|p=="self"){cursor=1;}
    else {while path.get(cursor).is_some_and(|p|p=="super"){out.pop();cursor+=1;}}
    out.extend_from_slice(&path[cursor..]);out
}
impl Qualified {
 fn resolve(&self,module:&[String],path:&[String],seen:&mut BTreeSet<String>)->Option<String> {
    if path.is_empty(){return None}
    let stamp=format!("{}|{}",name_key(module),name_key(path));if !seen.insert(stamp){return None}
    if ["crate","self","super"].contains(&path[0].as_str()) {
        let full=imported_path(module,path);let key=name_key(&full);
        if self.definitions.contains_key(&key){return Some(key)}
        let (last,parent)=full.split_last()?;return self.resolve(parent,std::slice::from_ref(last),seen)
    }
    let mut full=module.to_vec();full.extend_from_slice(path);let key=name_key(&full);
    if self.definitions.contains_key(&key){return Some(key)}
    if let Some(names)=self.modules.get(&name_key(module)) {
        if let Some(import)=names.imports.get(&path[0]) {
            let mut full=import.clone();full.extend_from_slice(&path[1..]);
            let key=name_key(&full);if self.definitions.contains_key(&key){return Some(key)}
            if let Some((last,parent))=full.split_last(){if let Some(r)=self.resolve(parent,std::slice::from_ref(last),seen){return Some(r)}}
        }
        let mut answers=BTreeSet::new();
        for prefix in &names.globs {if let Some(r)=self.resolve(prefix,path,seen){answers.insert(r);}}
        if answers.len()==1{return answers.into_iter().next()}
    }
    None
 }
 fn independent(&self,module:&[String],ty:&Type,bindings:&BTreeMap<String,(Vec<String>,Type)>,stack:&mut BTreeSet<String>)->bool {
    match ty {
      Type::Array(t)=>self.independent(module,&t.elem,bindings,stack),
      Type::Reference(t)=>self.independent(module,&t.elem,bindings,stack),
      Type::Slice(t)=>self.independent(module,&t.elem,bindings,stack),
      Type::Tuple(t)=>t.elems.iter().all(|t|self.independent(module,t,bindings,stack)),
      Type::Path(p) if p.qself.is_none()=>{
        let path:Vec<_>=p.path.segments.iter().map(|s|s.ident.to_string()).collect();let name=path.last().unwrap();
        if self.block_names.contains(name){return false}
        if path.len()==1 {if let Some((owner,value))=bindings.get(name){return self.independent(owner,value,&BTreeMap::new(),stack)}}
        if ["AccountInfo","Account","AccountLoader","InterfaceAccount","Signer","UncheckedAccount","Program","Context","CpiContext"].contains(&name.as_str()){return false}
        let args:Vec<Type>=match &p.path.segments.last().unwrap().arguments {PathArguments::None=>vec![],PathArguments::AngleBracketed(a)=>a.args.iter().filter_map(|a|if let GenericArgument::Type(t)=a{Some(t.clone())}else{None}).collect(),_=>return false};
        if let Some(key)=self.resolve(module,&path,&mut BTreeSet::new()) {
            let stack_key=format!("{}<{}>",key,args.iter().map(|x|x.to_token_stream().to_string()).collect::<Vec<_>>().join(","));if !stack.insert(stack_key.clone()){return false}
            let def=&self.definitions[&key];
            let answer=if def.params.len()==args.len(){let bindings=def.params.iter().cloned().zip(args.into_iter().map(|t|{
                if let Type::Path(p)=&t {if p.path.segments.len()==1 {if let Some(bound)=bindings.get(&p.path.segments[0].ident.to_string()){return bound.clone()}}}
                (module.to_vec(),t)
            })).collect();def.fields.iter().all(|t|self.independent(&def.module,t,&bindings,stack))}else{false};stack.remove(&stack_key);return answer
        }
        // Canonical prelude payload names only after no local/imported definition resolved.
        if path.len()==1&&self.modules.get(&name_key(module)).is_none_or(|m|m.imports.get(name).is_none()&&m.globs.iter().all(|g|g.iter().any(|p|p=="anchor_lang")||self.local_modules.contains(&name_key(g))))&&["u8","u16","u32","u64","u128","usize","i8","i16","i32","i64","i128","isize","bool","Pubkey"].contains(&name.as_str()){return args.is_empty()}
        if (path.len()==1||!self.local_modules.contains(&name_key(&[module, &path[..1]].concat())))&&self.modules.get(&name_key(module)).is_none_or(|m|m.imports.get(name).is_none_or(|p|p.iter().any(|x|["std","core","alloc"].contains(&x.as_str()))&&p.last()==Some(name))&&m.globs.iter().all(|g|g.iter().any(|p|p=="anchor_lang")||self.local_modules.contains(&name_key(g))))&&["Vec","Option","PhantomData"].contains(&name.as_str()) && (path.len()==1||["std","core","alloc"].contains(&path[0].as_str())) {return !args.is_empty()&&args.iter().all(|a|self.independent(module,a,bindings,stack))}
        false
      },_=>false
    }
 }
}
fn close_generic_observations(root:&Path,files:&[(PathBuf,syn::File)],types:&mut Types) {
    fn uses(tree:&syn::UseTree,prefix:&mut Vec<String>,module:&[String],out:&mut ModuleNames){match tree {
      syn::UseTree::Path(p)=>{prefix.push(p.ident.to_string());uses(&p.tree,prefix,module,out);prefix.pop();},
      syn::UseTree::Group(g)=>for t in &g.items{uses(t,prefix,module,out)},
      syn::UseTree::Name(n)=>{let mut path=prefix.clone();path.push(n.ident.to_string());out.imports.insert(n.ident.to_string(),imported_path(module,&path));},
      syn::UseTree::Rename(n)=>{let mut path=prefix.clone();path.push(n.ident.to_string());out.imports.insert(n.rename.to_string(),imported_path(module,&path));},
      syn::UseTree::Glob(_)=>out.globs.push(imported_path(module,prefix)),
    }}
    fn module_for(root:&Path,path:&Path)->Vec<String>{let relative=path.strip_prefix(root).unwrap();let mut p:Vec<String>=relative.parent().unwrap().iter().map(|s|s.to_string_lossy().into()).collect();let stem=path.file_stem().unwrap().to_string_lossy();if stem!="lib"&&stem!="mod"&&stem!="main"{p.push(stem.into())}p}
    fn collect(items:&[Item],module:&[String],q:&mut Qualified){
      q.modules.entry(name_key(module)).or_default();q.local_modules.insert(name_key(module));
      for item in items {match item {
        Item::Struct(s)=>{let mut path=module.to_vec();path.push(s.ident.to_string());q.definitions.insert(name_key(&path),LocalDefinition{module:module.to_vec(),params:s.generics.type_params().map(|p|p.ident.to_string()).collect(),fields:s.fields.iter().map(|f|f.ty.clone()).collect()});},
        Item::Type(t)=>{let mut path=module.to_vec();path.push(t.ident.to_string());q.definitions.insert(name_key(&path),LocalDefinition{module:module.to_vec(),params:t.generics.type_params().map(|p|p.ident.to_string()).collect(),fields:vec![(*t.ty).clone()]});},
        Item::Enum(e)=>{let mut path=module.to_vec();path.push(e.ident.to_string());q.definitions.insert(name_key(&path),LocalDefinition{module:module.to_vec(),params:e.generics.type_params().map(|p|p.ident.to_string()).collect(),fields:e.variants.iter().flat_map(|v|v.fields.iter().map(|f|f.ty.clone())).collect()});},
        Item::Union(u)=>{let mut path=module.to_vec();path.push(u.ident.to_string());q.definitions.insert(name_key(&path),LocalDefinition{module:module.to_vec(),params:u.generics.type_params().map(|p|p.ident.to_string()).collect(),fields:u.fields.named.iter().map(|f|f.ty.clone()).collect()});},
        Item::Use(u)=>uses(&u.tree,&mut vec![],module,q.modules.get_mut(&name_key(module)).unwrap()),
        Item::Mod(m)=>if let Some((_,items))=&m.content{let mut sub=module.to_vec();sub.push(m.ident.to_string());collect(items,&sub,q)},_=>{}
      }}
    }
    struct BlockNames(BTreeSet<String>);
    impl<'ast> Visit<'ast> for BlockNames {
        fn visit_block(&mut self,b:&'ast syn::Block){for s in &b.stmts{if let syn::Stmt::Item(i)=s{let name=match i{Item::Struct(x)=>Some(&x.ident),Item::Enum(x)=>Some(&x.ident),Item::Union(x)=>Some(&x.ident),Item::Type(x)=>Some(&x.ident),_=>None};if let Some(n)=name{self.0.insert(n.to_string());}}}visit::visit_block(self,b)}
    }
    let mut block=BlockNames(BTreeSet::new());for (_,file) in files{block.visit_file(file)}
    let mut q=Qualified::default();q.block_names=block.0;for (path,file) in files {collect(&file.items,&module_for(root,path),&mut q)}
    // Candidate is intentionally scoped to local functions with one explicit
    // generic call within an impl. All instances of that impl's data type must
    // resolve to concrete independent payloads.
    struct Scan<'a>{model:&'a Qualified,module:Vec<String>,owner:Option<Type>,functions:BTreeMap<String,Vec<Vec<String>>>,function_params:BTreeMap<String,Vec<String>>,renamed:BTreeSet<String>,macros:Vec<String>,all_paths:Vec<(Vec<String>,syn::Path,bool)>,calls:Vec<(String,Type,Vec<String>,syn::Path)>,instances:Vec<(Vec<String>,Type,Option<Type>,BTreeSet<String>)>,generic_params:BTreeSet<String>}
    impl<'ast> Visit<'ast> for Scan<'_>{
      fn visit_macro(&mut self,m:&'ast syn::Macro){self.macros.push(m.tokens.to_string());visit::visit_macro(self,m)}
      fn visit_item_use(&mut self,u:&'ast syn::ItemUse){fn renamed(t:&syn::UseTree,out:&mut BTreeSet<String>){match t{syn::UseTree::Rename(r)=>{out.insert(r.ident.to_string());},syn::UseTree::Path(p)=>renamed(&p.tree,out),syn::UseTree::Group(g)=>for t in &g.items{renamed(t,out)},_=>{}}}renamed(&u.tree,&mut self.renamed);}
      fn visit_item_fn(&mut self,f:&'ast syn::ItemFn){self.functions.entry(f.sig.ident.to_string()).or_default().push(self.module.clone());self.function_params.insert(f.sig.ident.to_string(),f.sig.generics.type_params().map(|p|p.ident.to_string()).collect());visit::visit_item_fn(self,f)}
      fn visit_item_impl(&mut self,i:&'ast syn::ItemImpl){let old=self.owner.replace((*i.self_ty).clone());let old_params=self.generic_params.clone();self.generic_params.extend(i.generics.type_params().map(|p|p.ident.to_string()));for item in &i.items{self.visit_impl_item(item)}self.owner=old;self.generic_params=old_params;}
      fn visit_item_mod(&mut self,m:&'ast syn::ItemMod){if m.attrs.iter().any(|a|a.path().is_ident("cfg")&&a.to_token_stream().to_string().replace(' ',"")=="#[cfg(test)]"){return}if let Some((_,items))=&m.content{self.module.push(m.ident.to_string());for i in items{self.visit_item(i)}self.module.pop();}}
      fn visit_expr_path(&mut self,p:&'ast syn::ExprPath){
        self.all_paths.push((self.module.clone(),p.path.clone(),self.owner.is_some()));
        let mut prefix=p.path.clone();prefix.segments.clear();
        for segment in &p.path.segments {
            prefix.segments.push(segment.clone());
            let names:Vec<_>=prefix.segments.iter().map(|s|s.ident.to_string()).collect();
            if let Some(key)=self.model.resolve(&self.module,&names,&mut BTreeSet::new()) {
                if !self.model.definitions[&key].params.is_empty() {
                    let mut typ=Type::Path(syn::TypePath{qself:None,path:prefix.clone()});
                    if matches!(segment.arguments,PathArguments::None) {
                        if let Some(Type::Path(owner))=&self.owner {
                            let owner_names:Vec<_>=owner.path.segments.iter().map(|s|s.ident.to_string()).collect();
                            if self.model.resolve(&self.module,&owner_names,&mut BTreeSet::new()).as_ref()==Some(&key){typ=Type::Path(owner.clone())}
                        }
                    }
                    self.instances.push((self.module.clone(),typ,self.owner.clone(),self.generic_params.clone()));
                }
            }
        }
        if let Some(owner)=&self.owner{let last=p.path.segments.last().unwrap();if matches!(last.arguments,PathArguments::AngleBracketed(_)){self.calls.push((last.ident.to_string(),owner.clone(),self.module.clone(),p.path.clone()));}}
        visit::visit_expr_path(self,p)
      }
      fn visit_type_path(&mut self,p:&'ast syn::TypePath){if !matches!(p.path.segments.last().unwrap().arguments,PathArguments::None){self.instances.push((self.module.clone(),Type::Path(p.clone()),self.owner.clone(),self.generic_params.clone()))}visit::visit_type_path(self,p)}
    }
    let mut scan=Scan{model:&q,module:vec![],owner:None,functions:BTreeMap::new(),function_params:BTreeMap::new(),renamed:BTreeSet::new(),macros:vec![],all_paths:vec![],calls:vec![],instances:vec![],generic_params:BTreeSet::new()};for (path,file) in files {scan.module=module_for(root,path);scan.visit_file(file)}
    for (name,owner,module,call) in &scan.calls {
        let Some(functions)=scan.functions.get(name) else{continue};if functions.len()!=1||scan.renamed.contains(name)||scan.macros.iter().any(|m|m.split(|c:char|!c.is_alphanumeric()&&c!='_').any(|n|n==name)){continue}
        let function_module=&functions[0];
        // Every occurrence must be an explicit generic invocation in an impl.
        // Function values, inferred parameters and out-of-impl calls stay SDK.
        if scan.all_paths.iter().filter(|(_,p,_)|p.segments.last().is_some_and(|s|s.ident==name)).any(|(_,p,in_impl)|!in_impl||!matches!(p.segments.last().unwrap().arguments,PathArguments::AngleBracketed(_))){continue}
        let call_parts:Vec<_>=call.segments.iter().map(|s|s.ident.to_string()).collect();let resolved=imported_path(module,&call_parts);
        let mut function_path=function_module.clone();function_path.push(name.clone());if resolved!=function_path{continue}
        let Type::Path(owner)=owner else{continue};let owner_path:Vec<_>=owner.path.segments.iter().map(|s|s.ident.to_string()).collect();let Some(owner_key)=q.resolve(module,&owner_path,&mut BTreeSet::new())else{continue};let owner_def=&q.definitions[&owner_key];if owner_def.params.is_empty()||scan.macros.iter().any(|m|m.split(|c:char|!c.is_alphanumeric()&&c!='_').any(|n|n==owner_path.last().unwrap())){continue}
        if scan.calls.iter().any(|(n,o,m,p)|n==name&&(o.to_token_stream().to_string()!=Type::Path(owner.clone()).to_token_stream().to_string()||m!=module||p.to_token_stream().to_string()!=call.to_token_stream().to_string())){continue}
        let mut seen=0;let mut valid=true;
        for (context,instance,instance_owner,generic_params) in &scan.instances {
            let Type::Path(p)=instance else{continue};let path:Vec<_>=p.path.segments.iter().map(|s|s.ident.to_string()).collect();if q.resolve(context,&path,&mut BTreeSet::new()).as_ref()!=Some(&owner_key){continue}
            let PathArguments::AngleBracketed(a)=&p.path.segments.last().unwrap().arguments else{valid=false;break};
            let args:Vec<_>=a.args.iter().filter_map(|a|if let GenericArgument::Type(t)=a{Some(t)}else{None}).collect();
            if instance_owner.is_some()&&args.len()==owner_def.params.len()&&args.iter().zip(&owner_def.params).all(|(a,b)|a.to_token_stream().to_string()==*b&&generic_params.contains(b)){continue}
            if !q.independent(context,instance,&BTreeMap::new(),&mut BTreeSet::new()){valid=false;break}seen+=1;
        }
        if valid&&seen>0 {
            let PathArguments::AngleBracketed(args)=&call.segments.last().unwrap().arguments else{continue};
            let args:Vec<_>=args.args.iter().filter_map(|a|if let GenericArgument::Type(t)=a{Some(t.to_token_stream().to_string())}else{None}).collect();
            if args.len()==scan.function_params[name].len()&&args.iter().all(|a|owner_def.params.contains(a)){types.closed_generic.insert(name.clone());}
        }
    }
}

fn substitute(ty:&Type, bindings:&BTreeMap<String,Type>)->Type {
    if let Type::Path(p)=ty {
        if p.qself.is_none()&&p.path.segments.len()==1 {
            if let Some(value)=bindings.get(&p.path.segments[0].ident.to_string()){return value.clone()}
        }
    }
    let mut out=ty.clone();
    match &mut out {
        Type::Array(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Slice(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Reference(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Ptr(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Paren(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Group(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Tuple(t)=>{for elem in &mut t.elems{*elem=substitute(elem,bindings)}},
        Type::Path(t)=>{for seg in &mut t.path.segments{if let PathArguments::AngleBracketed(a)=&mut seg.arguments{for arg in &mut a.args{if let GenericArgument::Type(t)=arg{*t=substitute(t,bindings)}}}}},
        _=>{}
    }
    out
}

fn observation(stream: TokenStream, types: &Types) -> Option<String> {
    // Inspect tokens in attributes/macros too; comments and string contents
    // never become observation identifiers.
    let tokens: Vec<_> = stream.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(g) = token {
            if let Some(reason) = observation(g.stream(), types) {
                return Some(reason);
            }
        }
        let TokenTree::Ident(ident) = token else {
            continue;
        };
        let local = ident.to_string();
        let name = types.observations.get(&local).cloned().unwrap_or(local);
        if ![
            "size_of",
            "align_of",
            "type_name",
            "TypeId",
            "transmute",
            "transmute_copy",
            "size_of_val",
            "align_of_val",
            "addr_of",
            "addr_of_mut",
            "downcast",
            "downcast_ref",
            "downcast_mut",
            "Any",
            "Rc",
            "RefCell",
        ]
        .contains(&name.as_str())
        {
            continue;
        }
        if [
            "addr_of",
            "addr_of_mut",
            "downcast",
            "downcast_ref",
            "downcast_mut",
            "Any",
            "size_of_val",
            "align_of_val",
            "Rc",
            "RefCell",
        ]
        .contains(&name.as_str())
        {
            return Some(format!(
                "representation observation {name} with unresolved value/type"
            ));
        }
        let mut at = index + 1;
        if name == "TypeId" {
            while at < tokens.len() && !matches!(&tokens[at],TokenTree::Ident(id)if id=="of") {
                at += 1
            }
            if at == tokens.len() {
                return Some("TypeId observation with unresolved type".into());
            }
            at += 1;
        }
        while at < tokens.len() && matches!(&tokens[at],TokenTree::Punct(p)if p.as_char()==':') {
            at += 1
        }
        if !matches!(tokens.get(at),Some(TokenTree::Punct(p))if p.as_char()=='<') {
            return Some(format!(
                "representation observation {name} with unresolved type"
            ));
        }
        let mut depth = 0;
        let mut generic = TokenStream::new();
        let mut end = at;
        for (offset, token) in tokens[at..].iter().enumerate() {
            match token {
                TokenTree::Punct(p) if p.as_char() == '<' => depth += 1,
                TokenTree::Punct(p) if p.as_char() == '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = at + offset;
                        break;
                    }
                }
                _ => {}
            }
            generic.extend([token.clone()]);
        }
        if depth != 0 {
            return Some(format!(
                "representation observation {name} with unparsed type"
            ));
        }
        generic.extend([tokens[end].clone()]);
        let arguments: syn::AngleBracketedGenericArguments = match syn::parse2(generic) {
            Ok(a) => a,
            Err(_) => {
                return Some(format!(
                    "representation observation {name} with unparsed type"
                ))
            }
        };
        let mut seen = false;
        for arg in arguments.args {
            if let GenericArgument::Type(ty) = arg {
                seen = true;
                if !types.independent(&ty, &mut BTreeSet::new()) {
                    return Some(format!(
                        "representation observation {name}<{}>",
                        ty.to_token_stream()
                    ));
                }
            }
        }
        if !seen {
            return Some(format!(
                "representation observation {name} with unresolved type"
            ));
        }
    }
    None
}
struct Inspect<'a> {
    types: &'a Types,
    reason: Option<String>,
}
impl<'ast> Visit<'ast> for Inspect<'_> {
    fn visit_item_mod(&mut self,node:&'ast syn::ItemMod) {
        let only_test=node.attrs.iter().any(|a|a.path().is_ident("cfg")&&a.to_token_stream().to_string().replace(' ',"")=="#[cfg(test)]");
        if !only_test {visit::visit_item_mod(self,node)}
    }
    fn visit_item_fn(&mut self,node:&'ast syn::ItemFn) {
        let only_test=node.attrs.iter().any(|a|a.path().is_ident("test")||(a.path().is_ident("cfg")&&a.to_token_stream().to_string().replace(' ',"")=="#[cfg(test)]"));
        if !only_test {
            if self.types.closed_generic.contains(&node.sig.ident.to_string()) {
                let mut local=self.types.clone();
                for p in node.sig.generics.type_params(){local.definitions.insert(p.ident.to_string(),Some(Definition::ProvenIndependent));}
                local.closed_generic.remove(&node.sig.ident.to_string());
                let mut inner=Inspect{types:&local,reason:None};visit::visit_item_fn(&mut inner,node);if self.reason.is_none(){self.reason=inner.reason}
            } else {visit::visit_item_fn(self,node)}
        }
    }

    fn visit_item_impl(&mut self, node:&'ast syn::ItemImpl) {
        let mut local=self.types.clone();
        local.definitions.insert("Self".into(),Some(Definition::Alias((*node.self_ty).clone())));
        let mut inner=Inspect {types:&local,reason:None};
        visit::visit_item_impl(&mut inner,node);
        if self.reason.is_none(){self.reason=inner.reason}
    }
    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        if self.reason.is_none() {
            self.reason = observation(node.meta.to_token_stream(), self.types)
        }
        visit::visit_attribute(self, node)
    }
    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if self.reason.is_none() {
            self.reason = observation(node.to_token_stream(), self.types)
        }
        visit::visit_expr_path(self, node)
    }
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if self.reason.is_none() {
            self.reason = observation(node.func.to_token_stream(), self.types)
        }
        visit::visit_expr_call(self, node)
    }
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if self.reason.is_none() && node.method.to_string().starts_with("downcast") {
            self.reason = Some("representation observation downcast".into())
        }
        visit::visit_expr_method_call(self, node)
    }
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if self.reason.is_none()
            && node
                .path
                .segments
                .last()
                .is_some_and(|s| s.ident == "addr_of" || s.ident == "addr_of_mut")
        {
            self.reason = Some("representation observation addr_of".into())
        }
        if self.reason.is_none() {
            self.reason = observation(node.tokens.clone(), self.types)
        }
        visit::visit_macro(self, node)
    }
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        if self.reason.is_none() && node.path.segments.last().is_some_and(|s| s.ident == "Any") {
            self.reason = Some("representation observation dyn Any".into())
        }
        visit::visit_type_path(self, node)
    }
}
fn dependency_metadata(source:&Path)->Option<serde_json::Value> {
    static CACHE:std::sync::OnceLock<std::sync::Mutex<BTreeMap<PathBuf,Option<serde_json::Value>>>>=std::sync::OnceLock::new();
    let manifest=source.ancestors().map(|p|p.join("Cargo.toml")).find(|p|p.exists())?;
    let cache=CACHE.get_or_init(||std::sync::Mutex::new(BTreeMap::new()));
    if let Some(value)=cache.lock().ok()?.get(&manifest).cloned(){return value}
    let output=std::process::Command::new("cargo").args(["metadata","--offline","--locked","--format-version","1","--manifest-path"]).arg(&manifest).output().ok()?;
    let value=if output.status.success(){serde_json::from_slice(&output.stdout).ok()}else{None};
    cache.lock().ok()?.insert(manifest,value.clone());value
}

fn resolve_local_payload_imports(source:&Path, files:&[(PathBuf,syn::File)],types:&mut Types) {
    let uses_pinocchio=files.iter().any(|(_,file)|file.items.iter().any(|i|matches!(i,Item::Use(u) if u.to_token_stream().to_string().replace(' ',"")=="pubusepinocchio::pubkey::Pubkey;")));
    if !uses_pinocchio {return}
    let Some(metadata)=dependency_metadata(source) else{return};
    let Some(packages)=metadata["packages"].as_array() else{return};
    let packages:Vec<_>=packages.iter().filter(|p|p["name"].as_str()==Some("pinocchio")).collect();
    if packages.len()!=1{return}
    let Some(manifest)=packages[0]["manifest_path"].as_str() else{return};
    let Some(root)=Path::new(manifest).parent() else{return};
    let Ok(text)=fs::read_to_string(root.join("src/pubkey.rs")) else{return};
    let compact=text.split_whitespace().collect::<String>();
    if !compact.contains("pubtypePubkey=[u8;PUBKEY_BYTES];")||!compact.contains("PUBKEY_BYTES:usize=32;"){return}
    let no_local_key=files.iter().all(|(_,f)|f.items.iter().all(|i|!matches!(i,Item::Struct(x) if x.ident=="Pubkey")&&!matches!(i,Item::Enum(x) if x.ident=="Pubkey")&&!matches!(i,Item::Type(x) if x.ident=="Pubkey")));
    if no_local_key {types.payload_imports.insert("Pubkey".into(),syn::parse_quote!([u8;32]));}
}

// Resolve this exact public reexport through the workspace's locked packages.
// Unknown imports remain unknown; no bare type-name exemption is installed.
fn resolve_external_payloads(source:&Path, files:&[(PathBuf,syn::File)], types:&mut Types) {
    let uses_stake=files.iter().any(|(_,f)|f.items.iter().any(|i|matches!(i,Item::Use(u) if u.to_token_stream().to_string().replace(' ',"")=="useanchor_lang::solana_program::stake::state::StakeState;")));
    if !uses_stake {return}
    let Some(metadata)=dependency_metadata(source) else{return};
    let Some(packages)=metadata["packages"].as_array() else{return};
    let package=|name:&str| -> Option<PathBuf> {
        let matches:Vec<_>=packages.iter().filter(|p|p["name"].as_str()==Some(name)).collect();
        if matches.len()!=1{return None}
        Some(PathBuf::from(matches[0]["manifest_path"].as_str()?).parent()?.to_path_buf())
    };
    let (Some(anchor),Some(program),Some(stake),Some(clock))=(package("anchor-lang"),package("solana-program"),package("solana-stake-interface"),package("solana-clock")) else{return};
    let (Ok(anchor_text),Ok(program_text),Ok(stake_text),Ok(clock_text))=(fs::read_to_string(anchor.join("src/lib.rs")),fs::read_to_string(program.join("src/stake.rs")),fs::read_to_string(stake.join("src/state.rs")),fs::read_to_string(clock.join("src/lib.rs"))) else{return};
    if !anchor_text.replace(' ',"").contains("pubusesolana_program;") || !program_text.contains("solana_stake_interface") || !program_text.contains("state") {return}
    let compact_clock=clock_text.split_whitespace().collect::<String>();
    if !compact_clock.contains("pubtypeEpoch=u64;")||!compact_clock.contains("pubtypeUnixTimestamp=i64;"){return}
    let Ok(file)=syn::parse_file(&stake_text) else{return};
    let mut external=Types::collect(&[(stake.join("src/state.rs"),file)]);
    external.definitions.remove("Pubkey");
    external.definitions.insert("Epoch".into(),Some(Definition::Alias(syn::parse_quote!(u64))));
    external.definitions.insert("UnixTimestamp".into(),Some(Definition::Alias(syn::parse_quote!(i64))));
    if external.independent(&syn::parse_quote!(StakeState),&mut BTreeSet::new()) {
        // Refuse ambiguity with any local declaration, even if another module imports the SDK type.
        if !files.iter().any(|(_,f)|f.items.iter().any(|i|matches!(i,Item::Struct(x) if x.ident=="StakeState")||matches!(i,Item::Enum(x) if x.ident=="StakeState")||matches!(i,Item::Type(x) if x.ident=="StakeState"))) {
            types.definitions.insert("StakeState".into(),Some(Definition::ProvenIndependent));
        }
    }
}

/// Independent local payload observations allow the raw type-check probe.
/// Unknown/generic types stay SDK; real SDK type boundaries are still decided
/// by the existing compile-and-check step after this source scan.
pub fn account_layout_observation(source: &Path) -> Result<Option<String>> {
    fn collect(path:&Path, files:&mut Vec<(PathBuf,syn::File)>)->Result<()> {
        let text=fs::read_to_string(path)?;
        let file=syn::parse_file(&text).with_context(||format!("parse {}",path.file_name().unwrap().to_string_lossy()))?;
        fn modules(items:&[Item],directory:&Path,files:&mut Vec<(PathBuf,syn::File)>)->Result<()> {
            for item in items {
                if let Item::Mod(module)=item {
                    if module.attrs.iter().any(|a|a.path().is_ident("cfg")&&a.to_token_stream().to_string().replace(' ',"")=="#[cfg(test)]") {continue}
                    if let Some((_,inline))=&module.content {modules(inline,&directory.join(module.ident.to_string()),files)?;continue}
                    let explicit=module.attrs.iter().find_map(|a|if a.path().is_ident("path"){if let syn::Meta::NameValue(n)=&a.meta{if let syn::Expr::Lit(e)=&n.value{if let syn::Lit::Str(p)=&e.lit{return Some(directory.join(p.value()))}}}None}else{None});
                    let stem=directory.join(module.ident.to_string());
                    let path=explicit.unwrap_or_else(||if stem.with_extension("rs").exists(){stem.with_extension("rs")}else{stem.join("mod.rs")});
                    if path.exists(){collect(&path,files)?}
                }
            }
            Ok(())
        }
        let directory=if path.file_name().is_some_and(|n|n=="lib.rs"||n=="main.rs"||n=="mod.rs"){path.parent().unwrap().to_path_buf()}else{path.with_extension("")};
        modules(&file.items,&directory,files)?;
        files.push((path.to_path_buf(),file));Ok(())
    }
    let mut files=vec![];
    let root=source.parent().context("program source directory")?;
    collect(source,&mut files)?;
    let mut types = Types::collect(&files);
    resolve_external_payloads(source,&files,&mut types);
    resolve_local_payload_imports(source,&files,&mut types);
    close_generic_observations(root,&files,&mut types);
    for (path, file) in files {
        // An unrelated module's glob import cannot obscure this module's bindings.
        let mut local=types.clone();
        local.external_glob=Types::collect(&[(path.clone(),file.clone())]).external_glob;
        let mut inspect = Inspect {
            types: &local,
            reason: None,
        };
        inspect.visit_file(&file);
        if let Some(reason) = inspect.reason {
            return Ok(Some(format!(
                "{reason} at {}",
                path.strip_prefix(root).unwrap_or(&path).display()
            )));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sdk(source: &str) -> bool {
        let file = syn::parse_file(source).unwrap();
        let files = vec![(PathBuf::from("lib.rs"), file)];
        let types = Types::collect(&files);
        let mut inspect = Inspect {
            types: &types,
            reason: None,
        };
        inspect.visit_file(&files[0].1);
        inspect.reason.is_some()
    }
    #[test]
    fn payload_layout_is_independent() {
        assert!(!sdk(
            "struct MyAccount { field:u8 } fn f(){let _=core::mem::size_of::<MyAccount>();}"
        ));
        assert!(!sdk("type Key=Pubkey; struct Inner{a:[u64;8],key:Key} struct Outer{v:Option<Vec<Inner>>} fn f(){let _=std::mem::align_of::<Outer>();}"));
    }
    #[test]
    fn account_layout_stays_sdk() {
        assert!(sdk(
            "struct Wrapper<'a>{a:AccountInfo<'a>} fn f(){let _=core::mem::size_of::<Wrapper>();}"
        ));
        assert!(sdk(
            "type Alias<'a>=AccountInfo<'a>; fn f(){let _=core::mem::size_of::<Alias>();}"
        ));
        assert!(sdk("fn f<T>(){let _=core::mem::size_of::<T>();}"));
    }
    #[test]
    fn imported_observer_alias_is_detected() {
        assert!(sdk(
            "use core::mem::size_of as size; fn f(){let _=size::<AccountInfo>();}"
        ));
        assert!(!sdk(
            "use core::mem::size_of as size; struct Data{a:u8} fn f(){let _=size::<Data>();}"
        ));
        assert!(sdk(
            "fn f(){use core::mem::size_of as s;let _=s::<AccountInfo>();}"
        ));
    }
    #[test]
    fn attribute_layout_is_scanned() {
        assert!(!sdk(
            "#[account(space=8+size_of::<Data>())] struct Context{} struct Data{value:u64}"
        ));
        assert!(sdk(
            "#[account(space=8+size_of::<AccountInfo>())] struct Context{}"
        ));
    }
    #[test]
    fn unknown_paths_stay_sdk() {
        assert!(sdk("fn f(){let _=size_of::<external::u8>();}"));
        assert!(sdk(
            "struct Generic<T>{x:T} fn f<T>(){let _=size_of::<Generic<T>>();}"
        ));
        assert!(sdk("fn f(v:&u8){let _=size_of_val(v);}"));
    }
    #[test]
    fn comments_and_strings_are_not_observations() {
        assert!(!sdk("// size_of AccountInfo RefCell
fn f(){let _=\"size_of::<AccountInfo>()\";}"));
    }
}

#[cfg(test)]
mod shadow_tests {
    use super::*;
    fn sdk(s: &str) -> bool {
        let f = syn::parse_file(s).unwrap();
        let types = Types::collect(&[(PathBuf::from("x.rs"), f.clone())]);
        let mut v = Inspect {
            types: &types,
            reason: None,
        };
        v.visit_file(&f);
        v.reason.is_some()
    }
    #[test]
    fn local_known_names_are_not_assumed_builtin() {
        assert!(sdk(
            "type Pubkey=AccountInfo; fn f(){let _=size_of::<Pubkey>();}"
        ));
        assert!(sdk(
            "type Vec<T>=AccountInfo; fn f(){let _=size_of::<Vec<u8>>();}"
        ));
    }
    #[test]
    fn unrelated_qualified_local_name_is_unknown() {
        assert!(sdk(
            "struct Payload{a:u8} fn f(){let _=size_of::<other::Payload>();}"
        ));
    }
    #[test]
    fn refcount_and_borrow_operations_are_conservative() {
        assert!(sdk("fn f(x:Rc<AccountInfo>){let _=Rc::strong_count(&x);}"));
        assert!(sdk("fn f(){let _=RefCell::new(0);}"));
        assert!(sdk(
            "use std::rc::Rc as R;fn f(x:R<u8>){let _=R::strong_count(&x);}"
        ));
    }
}

#[cfg(test)]
mod observer_value_tests {
    use super::*;
    fn sdk(s: &str) -> bool {
        let file = syn::parse_file(s).unwrap();
        let types = Types::collect(&[(PathBuf::from("x.rs"), file.clone())]);
        let mut v = Inspect {
            types: &types,
            reason: None,
        };
        v.visit_file(&file);
        v.reason.is_some()
    }
    #[test]
    fn observer_function_values_are_scanned() {
        assert!(sdk("fn f(){let s=std::mem::size_of::<AccountInfo>;s();}"));
        assert!(!sdk(
            "struct Data{a:u8} fn f(){let s=std::mem::size_of::<Data>;s();}"
        ));
    }
    #[test]
    fn address_macros_are_scanned() {
        assert!(sdk("fn f(x:AccountInfo){let _=core::ptr::addr_of!(x);}"));
    }
    #[test]
    fn generic_payloads_and_cycles_are_unknown() {
        assert!(sdk(
            "struct Empty<T>(std::marker::PhantomData<T>);fn f<T>(){let _=size_of::<Empty<T>>();}"
        ));
        assert!(sdk("type Cycle=Cycle;fn f(){let _=size_of::<Cycle>();}"));
    }
}

#[cfg(test)]
mod import_tests {
    use super::*;
    fn sdk(s: &str) -> bool {
        let f = syn::parse_file(s).unwrap();
        let types = Types::collect(&[(PathBuf::from("x.rs"), f.clone())]);
        let mut v = Inspect {
            types: &types,
            reason: None,
        };
        v.visit_file(&f);
        v.reason.is_some()
    }
    #[test]
    fn local_block_and_module_shadowing_is_unknown() {
        assert!(sdk(
            "fn f(){type Pubkey=AccountInfo;let _=size_of::<Pubkey>();}"
        ));
        assert!(sdk("mod std{pub mod mem{pub type u8=AccountInfo;}}fn f(){let _=size_of::<std::mem::u8>();}"));
    }
    #[test]
    fn external_import_collision_is_unknown() {
        assert!(sdk("mod a{pub struct Payload{value:u8}} mod b{use external::Payload;fn f(){let _=size_of::<Payload>();}}"));
        assert!(sdk(
            "use external::*;struct Payload{value:u8}fn f(){let _=size_of::<Payload>();}"
        ));
        assert!(sdk("use evil::Pubkey;fn f(){let _=size_of::<Pubkey>();}"));
        assert!(sdk(
            "use crate::m::Thing as Pubkey;fn f(){let _=size_of::<Pubkey>();}"
        ));
        assert!(sdk("fn f<String>(){let _=size_of::<String>();}"));
    }
}

#[cfg(test)]
mod resolved_payload_tests {
 use super::*;
 fn sdk(text:&str)->bool{let file=syn::parse_file(text).unwrap();let types=Types::collect(&[(PathBuf::from("lib.rs"),file.clone())]);let mut inspect=Inspect{types:&types,reason:None};inspect.visit_file(&file);inspect.reason.is_some()}
 #[test] fn concrete_self(){assert!(!sdk("struct P([u64;4]);impl P{fn bytes(){let _=size_of::<Self>();}}"));assert!(sdk("struct P(AccountInfo);impl P{fn bytes(){let _=size_of::<Self>();}}"));assert!(sdk("struct P<T>(T);impl<T> P<T>{fn bytes(){let _=size_of::<Self>();}}"));}
 #[test] fn uint_payload(){assert!(!sdk("use uint::construct_uint;construct_uint!{pub struct U256(4);} impl U256{fn f(){let _=size_of::<Self>();}}"));assert!(sdk("macro_rules! construct_uint{()=>{}} construct_uint!{pub struct U256(4);} impl U256{fn f(){let _=size_of::<Self>();}}"));}
}

#[cfg(test)]
mod generic_container_tests {
 use super::*;
 fn sdk(text:&str)->bool{let file=syn::parse_file(text).unwrap();let types=Types::collect(&[(PathBuf::from("lib.rs"),file.clone())]);let mut inspect=Inspect{types:&types,reason:None};inspect.visit_file(&file);inspect.reason.is_some()}
 #[test] fn closed_payload_container(){assert!(!sdk("struct Boxed<T>(Vec<T>); fn f(){let _=size_of::<Boxed<u64>>();}"));assert!(sdk("struct Boxed<T>(Vec<T>);fn f(){let _=size_of::<Boxed<AccountInfo>>();}"));assert!(sdk("struct Boxed<T>(Vec<T>);fn f<T>(){let _=size_of::<Boxed<T>>();}"));}
 #[test] fn local_macro_stays_unknown(){assert!(sdk("use uint::construct_uint; macro_rules! construct_uint{()=>{}} construct_uint!{pub struct U256(4);} impl U256{fn f(){let _=size_of::<Self>();}}"));}
}

#[cfg(test)]
mod closed_call_tests {
 use super::*;
 fn raw(text:&str)->bool {let file=syn::parse_file(text).unwrap();let files=vec![(PathBuf::from("lib.rs"),file.clone())];let mut types=Types::collect(&files);close_generic_observations(Path::new(""),&files,&mut types);let mut check=Inspect{types:&types,reason:None};check.visit_file(&file);check.reason.is_none()}
 const BASE:&str="struct Bag<L,T>(Vec<T>,PhantomData<L>);impl<L,T> Bag<L,T>{fn read(){hint::cautious::<T>();}}mod hint{pub fn cautious<T>(){let _=size_of::<T>();}}";
 #[test]fn closed_integer(){assert!(raw(&format!("{BASE} struct Data{{items:Bag<u8,u64>}}")));}
 #[test]fn wrapper_instance_rejected(){assert!(!raw(&format!("{BASE} struct Data{{items:Bag<u8,AccountInfo>}}")));}
 #[test]fn unknown_instance_rejected(){assert!(!raw(&format!("{BASE} struct Data<T>{{items:Bag<u8,T>}}")));}
 #[test]fn no_instance_rejected(){assert!(!raw(BASE));}
 #[test]fn generic_external_call_rejected(){assert!(!raw(&format!("{BASE} struct Data{{items:Bag<u8,u64>}}fn unknown<T>(){{hint::cautious::<T>();}}")));}
 #[test]fn alias_wrapper_rejected(){assert!(!raw(&format!("{BASE} type Hidden=AccountInfo;struct Data{{items:Bag<u8,Hidden>}}")));}
 #[test]fn function_alias_rejected(){assert!(!raw(&format!("{BASE} struct Data{{items:Bag<u8,u64>}}use hint::cautious as hidden;fn unknown<T>(){{hidden::<T>();}}")));}
 #[test]fn builtin_alias_shadow_rejected(){assert!(!raw(&format!("type Vec<T>=AccountInfo;{BASE} struct Data{{items:Bag<u8,u64>}}")));}
 #[test]fn macro_instance_rejected(){assert!(!raw(&format!("{BASE} struct Data{{items:Bag<u8,u64>}}make!(Bag<u8,AccountInfo>);")));}
 #[test]fn block_alias_shadow_rejected(){assert!(!raw(&format!("{BASE} struct Data{{items:Bag<u8,u64>}}fn f(){{type Vec<T>=AccountInfo;}}")));}
 #[test]fn external_glob_rejected(){assert!(!raw(&format!("use unknown::*;{BASE} struct Data{{items:Bag<u8,Pubkey>}}")));}
 #[test]fn expression_wrapper_instance_rejected(){assert!(!raw(&format!("{BASE} struct Data{{items:Bag<u8,u64>}}fn f(){{Bag::<u8,AccountInfo>::new();}}")));}
 #[test]fn nested_payload(){assert!(raw(&format!("{BASE} struct Nested{{items:Bag<u8,u64>}}struct Data{{items:Bag<u8,Nested>}}")));}
}

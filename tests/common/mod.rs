//! Oracle comparison helpers: compile with the official compiler (capnpc-embedded) and with
//! capnp-compiler, then compare the `CodeGeneratorRequest`s.
#![allow(dead_code)]

use capnp_compiler::schema_capnp::code_generator_request;
use capnp_compiler::{compile_files, request, CompileOptions};
use std::path::Path;

pub fn oracle_bytes(files: &[&str], src_prefix: &str, imports: &[&str]) -> Vec<u8> {
    let mut cmd = capnpc_embedded::CompileCommand::new();
    if !src_prefix.is_empty() {
        cmd.src_prefix(src_prefix);
    }
    for i in imports {
        cmd.import_path(i);
    }
    for f in files {
        cmd.file(f);
    }
    cmd.compile().expect("oracle compile")
}

pub fn mine_bytes(files: &[&str], src_prefix: &str, imports: &[&str]) -> Result<Vec<u8>, String> {
    let opts = CompileOptions {
        files: files.iter().map(|f| Path::new(f).to_path_buf()).collect(),
        src_prefixes: if src_prefix.is_empty() { vec![] } else { vec![src_prefix.into()] },
        import_paths: imports.iter().map(|i| i.into()).collect(),
        no_standard_import: false,
    };
    let out = compile_files(&opts).map_err(|e| e.to_string())?;
    Ok(request::request_bytes(&out))
}

fn read(bytes: &[u8]) -> capnp::message::Reader<capnp::serialize::OwnedSegments> {
    capnp::serialize::read_message(bytes, capnp::message::ReaderOptions::new()).unwrap()
}

#[derive(Default, Debug)]
pub struct Report {
    pub differences: Vec<String>,
    pub bytes_identical: bool,
}

/// Compare node by node (output order) and the rest of the request.
pub fn compare(files: &[&str], src_prefix: &str, imports: &[&str]) -> Report {
    let a = oracle_bytes(files, src_prefix, imports);
    let b = match mine_bytes(files, src_prefix, imports) {
        Ok(b) => b,
        Err(e) => {
            return Report { differences: vec![format!("compile error:\n{e}")], bytes_identical: false };
        }
    };
    let mut rep = Report { differences: vec![], bytes_identical: a == b };
    let ma = read(&a);
    let mb = read(&b);
    let ra: code_generator_request::Reader = ma.get_root().unwrap();
    let rb: code_generator_request::Reader = mb.get_root().unwrap();

    let na = ra.get_nodes().unwrap();
    let nb = rb.get_nodes().unwrap();
    if na.len() != nb.len() {
        rep.differences.push(format!("node count: oracle {} vs mine {}", na.len(), nb.len()));
    }
    for i in 0..na.len().min(nb.len()) {
        let sa = format!("{:#?}", na.get(i));
        let sb = format!("{:#?}", nb.get(i));
        if sa != sb {
            rep.differences.push(format!(
                "node #{i} ({}) differs:\n{}",
                na.get(i).get_display_name().unwrap().to_str().unwrap(),
                first_diff(&sa, &sb)
            ));
        }
    }
    // Order independent check.
    let mut ids_a: Vec<u64> = na.iter().map(|n| n.get_id()).collect();
    let mut ids_b: Vec<u64> = nb.iter().map(|n| n.get_id()).collect();
    if ids_a != ids_b {
        ids_a.sort();
        ids_b.sort();
        if ids_a == ids_b {
            rep.differences.push("node order differs (same set of ids)".into());
        } else {
            rep.differences.push("set of node ids differs".into());
        }
    }
    for (name, sa, sb) in [
        ("requestedFiles", format!("{:#?}", ra.get_requested_files().unwrap()), format!("{:#?}", rb.get_requested_files().unwrap())),
        ("capnpVersion", format!("{:#?}", ra.get_capnp_version().unwrap()), format!("{:#?}", rb.get_capnp_version().unwrap())),
    ] {
        if sa != sb {
            rep.differences.push(format!("{name} differs:\n{}", first_diff(&sa, &sb)));
        }
    }
    // sourceInfo: compared as a map by id first, then order.
    let mut sia: Vec<(u64, String)> =
        ra.get_source_info().unwrap().iter().map(|s| (s.get_id(), format!("{:#?}", s))).collect();
    let mut sib: Vec<(u64, String)> =
        rb.get_source_info().unwrap().iter().map(|s| (s.get_id(), format!("{:#?}", s))).collect();
    let order_a: Vec<u64> = sia.iter().map(|x| x.0).collect();
    let order_b: Vec<u64> = sib.iter().map(|x| x.0).collect();
    sia.sort();
    sib.sort();
    if sia != sib {
        let d = sia.iter().zip(sib.iter()).find(|(x, y)| x != y);
        rep.differences.push(format!(
            "sourceInfo content differs ({} vs {} entries): {:?}",
            sia.len(),
            sib.len(),
            d.map(|(x, y)| first_diff(&x.1, &y.1))
        ));
    } else if order_a != order_b {
        rep.differences.push("sourceInfo order differs".into());
    }
    rep
}

pub fn first_diff(a: &str, b: &str) -> String {
    let la: Vec<&str> = a.lines().collect();
    let lb: Vec<&str> = b.lines().collect();
    for i in 0..la.len().max(lb.len()) {
        if la.get(i) != lb.get(i) {
            let ctx_start = i.saturating_sub(6);
            let mut out = String::new();
            out.push_str("  context (oracle):\n");
            for l in &la[ctx_start..(i + 3).min(la.len())] {
                out.push_str(&format!("    {l}\n"));
            }
            out.push_str(&format!("  oracle: {:?}\n  mine:   {:?}\n", la.get(i), lb.get(i)));
            return out;
        }
    }
    "(identical text?)".into()
}

pub fn assert_same(files: &[&str], src_prefix: &str, imports: &[&str]) {
    let rep = compare(files, src_prefix, imports);
    if !rep.differences.is_empty() {
        panic!("{} differences:\n{}", rep.differences.len(), rep.differences.join("\n---\n"));
    }
}

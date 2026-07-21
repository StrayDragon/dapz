//! dapz Compression Benchmark Report
//!
//! Loads JSON fixtures from `fixtures/bench/`, runs interceptors, and:
//! 1. Writes the full report to `docs/src/benchmarks.md`
//! 2. Syncs the README summary block between `<!-- BENCH-SUMMARY:START/END -->`
//! 3. Prints the full report to stdout
//!
//! Run: `just gen-bench` / `just bench-report`

use std::fs;
use std::path::Path;

use serde_json::Value;
use tiktoken_rs::cl100k_base;

use dapz::codec::json_rpc::DapMessage;
use dapz::codec::toon::value_to_toon;
use dapz::interceptors::Interceptor;
use dapz::interceptors::capping::CappingInterceptor;
use dapz::interceptors::evaluate::EvaluateCompressor;
use dapz::interceptors::exception::ExceptionInfoCompressor;
use dapz::interceptors::output::OutputCompressor;
use dapz::interceptors::scopes::ScopesCompressor;
use dapz::interceptors::stacktrace::StackTraceCompressor;
use dapz::interceptors::variables::VariablesCompressor;
use dapz::proxy::Direction;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const BENCH_MD: &str = "docs/src/benchmarks.md";
const README_MD: &str = "README.md";
const BENCH_START: &str = "<!-- BENCH-SUMMARY:START -->";
const BENCH_END: &str = "<!-- BENCH-SUMMARY:END -->";

/// (fixture file, display name, README DAP-message column)
const COMPRESSORS: &[(&str, &str, &str)] = &[
    ("output.json", "OutputCompressor", "`output`"),
    ("variables.json", "VariablesCompressor", "`variables`"),
    ("stacktrace.json", "StackTraceCompressor", "`stackTrace`"),
    ("scopes.json", "ScopesCompressor", "`scopes`"),
    ("evaluate.json", "EvaluateCompressor", "`evaluate`"),
    (
        "exception.json",
        "ExceptionInfoCompressor",
        "`exceptionInfo`",
    ),
    ("capping.json", "CappingInterceptor", "大响应截断"),
];

fn get_today() -> String {
    let output = std::process::Command::new("date").arg("+%Y-%m-%d").output();
    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => "unknown".to_string(),
    }
}

struct FixtureCase {
    name: String,
    interceptor: String,
    message: DapMessage,
}

fn load_fixtures(path: &str) -> Vec<FixtureCase> {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", path, e);
        std::process::exit(1);
    });
    let cases: Vec<Value> = serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Error parsing {}: {}", path, e);
        std::process::exit(1);
    });
    cases
        .into_iter()
        .map(|c| {
            let msg: DapMessage =
                serde_json::from_value(c["message"].clone()).unwrap_or_else(|e| {
                    eprintln!("Error parsing message in {}: {}", path, e);
                    std::process::exit(1);
                });
            FixtureCase {
                name: c["name"].as_str().unwrap().to_string(),
                interceptor: c["interceptor"].as_str().unwrap().to_string(),
                message: msg,
            }
        })
        .collect()
}

fn make_interceptor(name: &str) -> Option<Box<dyn Interceptor>> {
    match name {
        "output_compressor" => Some(Box::new(OutputCompressor)),
        "evaluate_compressor" => Some(Box::new(EvaluateCompressor::new(500))),
        "variables_compressor" => Some(Box::new(VariablesCompressor::new(120))),
        "stacktrace_compressor" => Some(Box::new(StackTraceCompressor)),
        "scopes_compressor" => Some(Box::new(ScopesCompressor)),
        "exception_info_compressor" => Some(Box::new(ExceptionInfoCompressor::new(800))),
        "capping_interceptor" => Some(Box::new(CappingInterceptor::new(5, 5, 500))),
        _ => None,
    }
}

struct BenchResult {
    orig_tokens: usize,
    comp_tokens: usize,
    toon_tokens: usize,
}

struct GroupSummary {
    name: String,
    dap_msg: String,
    case_count: usize,
    orig: usize,
    comp: usize,
    toon: usize,
}

async fn bench_case(bpe: &tiktoken_rs::CoreBPE, case: &FixtureCase) -> Option<BenchResult> {
    let orig_json = serde_json::to_string(&case.message).unwrap();
    let orig_tokens = bpe.encode_with_special_tokens(&orig_json).len();

    let compressor = make_interceptor(&case.interceptor)?;
    let result = match compressor
        .intercept(case.message.clone(), Direction::ServerToClient)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) | Err(_) => return None,
    };

    let comp_json = serde_json::to_string(&result).unwrap();
    let comp_tokens = bpe.encode_with_special_tokens(&comp_json).len();

    let toon_value = result
        .body
        .clone()
        .unwrap_or_else(|| serde_json::to_value(&result).unwrap_or(Value::Null));
    let toon_tokens = match value_to_toon(&toon_value) {
        Ok(text) => bpe.encode_with_special_tokens(&text).len(),
        Err(_) => comp_tokens,
    };

    Some(BenchResult {
        orig_tokens,
        comp_tokens,
        toon_tokens,
    })
}

fn pct(a: usize, b: usize) -> f64 {
    if a == 0 {
        0.0
    } else {
        (a as f64 - b as f64) / a as f64 * 100.0
    }
}

fn build_full_report(
    detail_rows: &[(String, String, BenchResult)],
    groups: &[GroupSummary],
    grand: &(usize, usize, usize, usize),
) -> String {
    let (cases, g_orig, g_comp, g_toon) = *grand;
    let mut out = String::new();
    out.push_str(
        "<!-- AUTO-GENERATED by examples/bench-report.rs. Do not edit manually. Run: just gen-bench -->\n\n",
    );
    out.push_str("# dapz 压缩基准测试报告\n\n");
    out.push_str(&format!("**版本**: v{}\n\n", VERSION));
    out.push_str(&format!("**日期**: {}\n\n", get_today()));
    out.push_str(
        "测量标准：原始/紧凑 = 整条 DAP JSON（`cl100k_base`）；TOON = 压缩后 `body` 经 `value_to_toon`（对齐 MCP/SDK 出口）。\n\n",
    );
    out.push_str("| 压缩器 | 场景 | 原始 (T) | 紧凑 (T) | TOON (T) | Δ% 紧凑 | Δ% TOON |\n");
    out.push_str("|----------|------|----------|----------|----------|---------|---------|\n");
    for (eng, case_name, r) in detail_rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.1}% | {:.1}% |\n",
            eng,
            case_name,
            r.orig_tokens,
            r.comp_tokens,
            r.toon_tokens,
            pct(r.orig_tokens, r.comp_tokens),
            pct(r.orig_tokens, r.toon_tokens),
        ));
    }
    out.push_str("\n## 总结\n\n");
    out.push_str("| 压缩器 | 场景数 | 原始 (T) | 紧凑 (T) | TOON (T) | Δ% 紧凑 | Δ% TOON |\n");
    out.push_str("|----------|--------|----------|----------|----------|---------|---------|\n");
    for g in groups {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.1}% | {:.1}% |\n",
            g.name,
            g.case_count,
            g.orig,
            g.comp,
            g.toon,
            pct(g.orig, g.comp),
            pct(g.orig, g.toon),
        ));
    }
    out.push_str(&format!(
        "| **总体** | **{}** | **{}** | **{}** | **{}** | **{:.1}%** | **{:.1}%** |\n",
        cases,
        g_orig,
        g_comp,
        g_toon,
        pct(g_orig, g_comp),
        pct(g_orig, g_toon),
    ));
    out
}

fn build_readme_summary(groups: &[GroupSummary], grand: &(usize, usize, usize, usize)) -> String {
    let (cases, g_orig, g_comp, g_toon) = *grand;
    let mut out = String::new();
    out.push_str("| 拦截器 | DAP 消息 | 紧凑 vs 原始 | TOON vs 原始 |\n");
    out.push_str("|--------|---------|-------------|-------------|\n");
    for g in groups {
        out.push_str(&format!(
            "| {} | {} | {:.1}% | {:.1}% |\n",
            g.name,
            g.dap_msg,
            pct(g.orig, g.comp),
            pct(g.orig, g.toon),
        ));
    }
    out.push_str(&format!(
        "| **总体（{} 场景）** | — | **{:.1}%** | **{:.1}%** |\n\n",
        cases,
        pct(g_orig, g_comp),
        pct(g_orig, g_toon),
    ));
    out.push_str(
        "> 紧凑格式对小输入收益有限；TOON 在 MCP/SDK 路径下额外省 token。完整表：[docs/src/benchmarks.md](docs/src/benchmarks.md)（`just gen-bench` 同步更新本表与完整报告）；快速演示：`just compress-demo`。\n",
    );
    out
}

fn sync_readme_summary(summary: &str) {
    let readme = fs::read_to_string(README_MD).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", README_MD, e);
        std::process::exit(1);
    });
    let Some(start) = readme.find(BENCH_START) else {
        eprintln!(
            "Error: {} missing marker `{}` — add it under ## 压缩效果",
            README_MD, BENCH_START
        );
        std::process::exit(1);
    };
    let Some(end_rel) = readme[start..].find(BENCH_END) else {
        eprintln!("Error: {} missing marker `{}`", README_MD, BENCH_END);
        std::process::exit(1);
    };
    let end = start + end_rel;
    let mut next = String::new();
    next.push_str(&readme[..start]);
    next.push_str(BENCH_START);
    next.push('\n');
    next.push_str(summary);
    if !summary.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(&readme[end..]);
    fs::write(README_MD, next).unwrap_or_else(|e| {
        eprintln!("Error writing {}: {}", README_MD, e);
        std::process::exit(1);
    });
}

#[tokio::main]
async fn main() {
    let bpe = cl100k_base().expect("Failed to initialize tiktoken");
    let fixture_dir = Path::new("fixtures/bench");

    let mut detail_rows: Vec<(String, String, BenchResult)> = Vec::new();
    let mut groups: Vec<GroupSummary> = Vec::new();
    let mut grand_orig = 0usize;
    let mut grand_comp = 0usize;
    let mut grand_toon = 0usize;
    let mut grand_cases = 0usize;

    for (filename, eng_name, dap_msg) in COMPRESSORS {
        let path = fixture_dir.join(filename);
        let cases = load_fixtures(&path.to_string_lossy());
        let mut results: Vec<BenchResult> = Vec::new();

        for case in &cases {
            match bench_case(&bpe, case).await {
                Some(result) => {
                    results.push(BenchResult {
                        orig_tokens: result.orig_tokens,
                        comp_tokens: result.comp_tokens,
                        toon_tokens: result.toon_tokens,
                    });
                    detail_rows.push((eng_name.to_string(), case.name.clone(), result));
                }
                None => {
                    eprintln!("WARN  skipped {}:{}", eng_name, case.name);
                }
            }
        }

        if results.is_empty() {
            continue;
        }
        let orig: usize = results.iter().map(|r| r.orig_tokens).sum();
        let comp: usize = results.iter().map(|r| r.comp_tokens).sum();
        let toon: usize = results.iter().map(|r| r.toon_tokens).sum();
        grand_orig += orig;
        grand_comp += comp;
        grand_toon += toon;
        grand_cases += results.len();
        groups.push(GroupSummary {
            name: eng_name.to_string(),
            dap_msg: dap_msg.to_string(),
            case_count: results.len(),
            orig,
            comp,
            toon,
        });
    }

    let grand = (grand_cases, grand_orig, grand_comp, grand_toon);
    let full = build_full_report(&detail_rows, &groups, &grand);
    let summary = build_readme_summary(&groups, &grand);

    if let Some(parent) = Path::new(BENCH_MD).parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(BENCH_MD, &full).unwrap_or_else(|e| {
        eprintln!("Error writing {}: {}", BENCH_MD, e);
        std::process::exit(1);
    });
    sync_readme_summary(&summary);

    print!("{full}");
    eprintln!("Updated {BENCH_MD}");
    eprintln!("Updated {README_MD} ({BENCH_START} … {BENCH_END})");
}

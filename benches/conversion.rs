use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::NamedTempFile;

/// Representative Markdown document used across all benchmarks.
///
/// Contains: a heading, 3 paragraphs of Korean text, a 3×3 table,
/// a fenced code block, a 5-item bullet list, and bold/italic spans.
const SAMPLE_MD: &str = r##"# 문서 제목: 한국어 테스트 문서

이 문단은 첫 번째 단락으로, **굵은 글씨**와 *기울임꼴*이 포함되어 있습니다.
한국어로 작성된 다양한 텍스트를 변환 성능 측정에 활용합니다.

두 번째 단락에는 조금 더 긴 텍스트가 포함됩니다. 자연어 처리와 문서 변환에서
성능은 매우 중요한 요소입니다. 특히 대용량 문서를 다룰 때 변환 속도가 핵심입니다.

세 번째 단락은 **굵고** _기울임_ 조합 서식과 함께 작성되었으며,
문서 전반의 다양한 인라인 요소를 포함합니다.

| 항목 | 설명 | 비고 |
|------|------|------|
| 첫째 행 | 한국어 데이터 첫 번째 | 비고 A |
| 둘째 행 | 한국어 데이터 두 번째 | 비고 B |
| 셋째 행 | 한국어 데이터 세 번째 | 비고 C |

```rust
fn main() {
    println!("Hello, hwp2md!");
    let doc = hwp2md::md::parse_markdown("# Heading");
    println!("{doc:?}");
}
```

- 첫 번째 항목: 변환 정확도 검증
- 두 번째 항목: 성능 측정 기준 수립
- 세 번째 항목: **굵은** 인라인 서식 지원
- 네 번째 항목: *기울임* 인라인 서식 지원
- 다섯 번째 항목: 라운드트립 무결성 보장
"##;

/// Table-heavy Markdown document: 20-row × 5-column table for write/read benchmarks.
const TABLE_HEAVY_MD: &str = "\
| 항목 | 설명 | 값 | 단위 | 비고 |\n\
|------|------|-----|------|------|\n\
| 행 01 | 데이터 항목 1 | 100 | ms | 측정값 A |\n\
| 행 02 | 데이터 항목 2 | 200 | ms | 측정값 B |\n\
| 행 03 | 데이터 항목 3 | 300 | ms | 측정값 C |\n\
| 행 04 | 데이터 항목 4 | 400 | ms | 측정값 D |\n\
| 행 05 | 데이터 항목 5 | 500 | ms | 측정값 E |\n\
| 행 06 | 데이터 항목 6 | 600 | ms | 측정값 F |\n\
| 행 07 | 데이터 항목 7 | 700 | ms | 측정값 G |\n\
| 행 08 | 데이터 항목 8 | 800 | ms | 측정값 H |\n\
| 행 09 | 데이터 항목 9 | 900 | ms | 측정값 I |\n\
| 행 10 | 데이터 항목 10 | 1000 | ms | 측정값 J |\n\
| 행 11 | 데이터 항목 11 | 1100 | ms | 측정값 K |\n\
| 행 12 | 데이터 항목 12 | 1200 | ms | 측정값 L |\n\
| 행 13 | 데이터 항목 13 | 1300 | ms | 측정값 M |\n\
| 행 14 | 데이터 항목 14 | 1400 | ms | 측정값 N |\n\
| 행 15 | 데이터 항목 15 | 1500 | ms | 측정값 O |\n\
| 행 16 | 데이터 항목 16 | 1600 | ms | 측정값 P |\n\
| 행 17 | 데이터 항목 17 | 1700 | ms | 측정값 Q |\n\
| 행 18 | 데이터 항목 18 | 1800 | ms | 측정값 R |\n\
| 행 19 | 데이터 항목 19 | 1900 | ms | 측정값 S |\n\
| 행 20 | 데이터 항목 20 | 2000 | ms | 측정값 T |\n\
";

fn bench_md_to_ir(c: &mut Criterion) {
    c.bench_function("md_to_ir", |b| {
        b.iter(|| hwp2md::md::parse_markdown(black_box(SAMPLE_MD)));
    });
}

fn bench_ir_to_md(c: &mut Criterion) {
    let doc = hwp2md::md::parse_markdown(SAMPLE_MD);
    c.bench_function("ir_to_md", |b| {
        b.iter(|| hwp2md::md::write_markdown(black_box(&doc), false));
    });
}

fn bench_ir_to_hwpx(c: &mut Criterion) {
    let doc = hwp2md::md::parse_markdown(SAMPLE_MD);
    let tmp = NamedTempFile::new().expect("tempfile");
    c.bench_function("ir_to_hwpx", |b| {
        b.iter(|| {
            hwp2md::hwpx::write_hwpx(black_box(&doc), tmp.path(), None).expect("write_hwpx failed");
        });
    });
}

fn bench_hwpx_to_ir(c: &mut Criterion) {
    // Write once outside the loop; benchmark only the read path.
    let tmp = NamedTempFile::new().expect("tempfile");
    let doc = hwp2md::md::parse_markdown(SAMPLE_MD);
    hwp2md::hwpx::write_hwpx(&doc, tmp.path(), None).expect("write_hwpx failed");

    c.bench_function("hwpx_to_ir", |b| {
        b.iter(|| hwp2md::hwpx::read_hwpx(black_box(tmp.path())).expect("read_hwpx failed"));
    });
}

fn bench_roundtrip(c: &mut Criterion) {
    let tmp = NamedTempFile::new().expect("tempfile");
    c.bench_function("md_hwpx_md_roundtrip", |b| {
        b.iter(|| {
            let doc = hwp2md::md::parse_markdown(black_box(SAMPLE_MD));
            hwp2md::hwpx::write_hwpx(&doc, tmp.path(), None).expect("write_hwpx failed");
            let doc2 = hwp2md::hwpx::read_hwpx(tmp.path()).expect("read_hwpx failed");
            hwp2md::md::write_markdown(black_box(&doc2), false)
        });
    });
}

fn bench_ir_to_hwpx_table_heavy(c: &mut Criterion) {
    let doc = hwp2md::md::parse_markdown(TABLE_HEAVY_MD);
    let tmp = NamedTempFile::new().expect("tempfile");
    c.bench_function("ir_to_hwpx_table_heavy", |b| {
        b.iter(|| {
            hwp2md::hwpx::write_hwpx(black_box(&doc), tmp.path(), None).expect("write_hwpx failed");
        });
    });
}

fn bench_hwpx_table_heavy_roundtrip(c: &mut Criterion) {
    // Pre-parse MD outside the loop; benchmarks only the write+read path.
    let doc = hwp2md::md::parse_markdown(TABLE_HEAVY_MD);
    let tmp = NamedTempFile::new().expect("tempfile");
    c.bench_function("hwpx_table_heavy_write_read", |b| {
        b.iter(|| {
            hwp2md::hwpx::write_hwpx(black_box(&doc), tmp.path(), None).expect("write_hwpx");
            hwp2md::hwpx::read_hwpx(tmp.path()).expect("read_hwpx")
        });
    });
}

/// Benchmark HTML table parsing with a larger 100×10 table to measure
/// per-tag heap allocation cost in `local_name`.
fn bench_parse_html_table_large(c: &mut Criterion) {
    // Build a 100-row × 10-col HTML table string once outside the hot loop.
    let mut html = String::from("<table>");
    for _ in 0..100 {
        html.push_str("<tr>");
        for col in 0..10u8 {
            html.push_str("<td>cell");
            html.push(char::from(b'0' + col));
            html.push_str("</td>");
        }
        html.push_str("</tr>");
    }
    html.push_str("</table>");

    c.bench_function("parse_html_table_large_100x10", |b| {
        b.iter(|| hwp2md::md::html_table::parse_html_table(black_box(&html)));
    });
}

criterion_group!(
    benches,
    bench_md_to_ir,
    bench_ir_to_md,
    bench_ir_to_hwpx,
    bench_hwpx_to_ir,
    bench_roundtrip,
    bench_ir_to_hwpx_table_heavy,
    bench_hwpx_table_heavy_roundtrip,
    bench_parse_html_table_large
);
criterion_main!(benches);

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

fn bench_md_to_ir(c: &mut Criterion) {
    c.bench_function("md_to_ir", |b| {
        b.iter(|| hwp2md::md::parse_markdown(black_box(SAMPLE_MD)))
    });
}

fn bench_ir_to_md(c: &mut Criterion) {
    let doc = hwp2md::md::parse_markdown(SAMPLE_MD);
    c.bench_function("ir_to_md", |b| {
        b.iter(|| hwp2md::md::write_markdown(black_box(&doc), false))
    });
}

fn bench_ir_to_hwpx(c: &mut Criterion) {
    let doc = hwp2md::md::parse_markdown(SAMPLE_MD);
    c.bench_function("ir_to_hwpx", |b| {
        b.iter(|| {
            let tmp = NamedTempFile::new().expect("tempfile");
            hwp2md::hwpx::write_hwpx(black_box(&doc), tmp.path(), None)
                .expect("write_hwpx failed");
        })
    });
}

fn bench_hwpx_to_ir(c: &mut Criterion) {
    // Write once outside the loop; benchmark only the read path.
    let tmp = NamedTempFile::new().expect("tempfile");
    let doc = hwp2md::md::parse_markdown(SAMPLE_MD);
    hwp2md::hwpx::write_hwpx(&doc, tmp.path(), None).expect("write_hwpx failed");

    c.bench_function("hwpx_to_ir", |b| {
        b.iter(|| hwp2md::hwpx::read_hwpx(black_box(tmp.path())).expect("read_hwpx failed"))
    });
}

fn bench_roundtrip(c: &mut Criterion) {
    c.bench_function("md_hwpx_md_roundtrip", |b| {
        b.iter(|| {
            // MD → IR
            let doc = hwp2md::md::parse_markdown(black_box(SAMPLE_MD));
            // IR → HWPX (temporary file)
            let tmp = NamedTempFile::new().expect("tempfile");
            hwp2md::hwpx::write_hwpx(&doc, tmp.path(), None).expect("write_hwpx failed");
            // HWPX → IR
            let doc2 = hwp2md::hwpx::read_hwpx(tmp.path()).expect("read_hwpx failed");
            // IR → MD
            hwp2md::md::write_markdown(black_box(&doc2), false)
        })
    });
}

criterion_group!(
    benches,
    bench_md_to_ir,
    bench_ir_to_md,
    bench_ir_to_hwpx,
    bench_hwpx_to_ir,
    bench_roundtrip
);
criterion_main!(benches);

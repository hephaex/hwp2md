# hwp2md

[![Crates.io](https://img.shields.io/crates/v/hwp2md.svg)](https://crates.io/crates/hwp2md)
[![CI](https://github.com/hephaex/hwp2md/actions/workflows/ci.yml/badge.svg)](https://github.com/hephaex/hwp2md/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/hephaex/hwp2md/graph/badge.svg)](https://codecov.io/gh/hephaex/hwp2md)
[![License: GPL-3.0-only](https://img.shields.io/badge/license-GPL--3.0--only-blue.svg)](LICENSE)

> [English](README.en.md)

**hwp2md**는 한국 한글 문서 형식 — HWP 5.0 (바이너리 OLE2)과 HWPX (XML/ZIP) — 을 CommonMark 호환 Markdown으로 양방향 변환하는 도구입니다. CLI와 Rust 라이브러리 두 가지 형태로 제공되며, 빌드 파이프라인, 정적 사이트 생성기, 공공기관 문서 관리 워크플로우에 쉽게 통합할 수 있습니다.

## 주요 기능

- **HWP 5.0** 바이너리 형식 (OLE2/CFB 컨테이너) → Markdown
- **HWPX** (ZIP + XML) → Markdown
- **Markdown → HWPX** (바이너리 HWP 출력은 미지원)
- 제목(H1-H6), 문단, 굵게, 기울임, 밑줄, 취소선, 인라인 코드
- 위첨자 / 아래첨자
- 하이퍼링크 (`fieldBegin`/`fieldEnd` 패턴)
- 루비 주석 (`<ruby>본문<rt>주석</rt></ruby>`)
- 각주 참조 (`hp:noteRef`)
- 인라인 코드 → HWPX에서 고정폭 글꼴(Courier New)로 매핑
- 메타데이터(제목, 저자) HWPX `hp:docInfo` 라운드트립
- 순서/비순서 목록 (중첩 지원)
- 표 (GFM 파이프 구문; colspan/rowspan은 HTML 폴백)
- 코드 블록 (언어 주석 포함)
- 인용 블록
- 이미지 (alt 텍스트; 에셋 디렉토리 추출 옵션)
- 각주 (`[^id]` 구문)
- 수식 — HWP EqEdit 수식을 LaTeX로 변환 (`$...$` / `$$...$$`)
- 문서 메타데이터 YAML 프론트매터 출력
- `info` 서브커맨드 — 변환 없이 문서 정보 확인
- YAML 스타일 템플릿 (용지 크기, 여백, 글꼴, 제목 줄간격)
- 구조화된 중간 표현(IR)을 퍼블릭 라이브러리 API로 제공
- LTO + 심볼 스트립으로 최소 바이너리 크기

## 설치

### crates.io에서 설치

```bash
cargo install hwp2md
```

### 소스에서 빌드

```bash
git clone https://github.com/hephaex/hwp2md.git
cd hwp2md
cargo build --release
# 바이너리: target/release/hwp2md
```

최소 Rust 버전: **1.88**

## CLI 사용법

### HWP/HWPX → Markdown 변환

```bash
# 표준 출력으로 Markdown 출력
hwp2md to-md report.hwp

# 파일로 저장
hwp2md to-md report.hwp -o report.md

# 이미지 추출과 함께 변환
hwp2md to-md report.hwpx -o report.md --assets-dir ./images

# YAML 프론트매터 포함
hwp2md to-md report.hwp -o report.md --frontmatter
```

### Markdown → HWPX 변환

```bash
# 출력 파일명 자동 결정 (.hwpx 확장자)
hwp2md to-hwpx draft.md

# 출력 경로 지정
hwp2md to-hwpx draft.md -o final.hwpx

# YAML 스타일 템플릿 적용
hwp2md to-hwpx draft.md -o final.hwpx --style corporate.yaml
```

### 자동 형식 감지 (`convert`)

```bash
# 확장자로 방향 자동 추론 (.hwpx → .md)
hwp2md convert report.hwpx report.md

# 옵션: 프론트매터, 이미지 추출, 스타일 템플릿
hwp2md convert report.hwpx report.md --frontmatter --assets-dir ./images
hwp2md convert draft.md draft.hwpx --style corporate.yaml

# 기존 출력 파일 덮어쓰기
hwp2md convert report.hwpx report.md --force
```

### 디렉토리 일괄 변환

```bash
# 디렉토리 내 모든 HWP/HWPX 파일 변환
hwp2md batch ./reports/ --output-dir converted/

# 파일별 이미지 서브디렉토리 추출
hwp2md batch ./reports/ --assets-dir ./images --frontmatter

# 기존 출력 파일 덮어쓰기
hwp2md batch ./reports/ -o converted/ --force
```

### 문서 정보 확인

```bash
hwp2md info report.hwp
# File: report.hwp
# Format: hwp
# Title: 연간 보고서 2025
# Author: 홍길동
# Sections: 4
# Blocks: 87
# Characters: ~12430
# Assets: 6
```

### 로그 수준 설정

`--log-level` 플래그에 `tracing` 필터 문자열 사용 (기본값: `info`):

```bash
hwp2md --log-level debug to-md report.hwp
hwp2md --log-level warn  to-md report.hwp -o report.md
```

## 라이브러리 사용법

`Cargo.toml`에 추가:

```toml
[dependencies]
hwp2md = "0.5"
```

### 파일 변환

```rust
use hwp2md::convert;

fn main() -> anyhow::Result<()> {
    // HWP/HWPX → Markdown (output이 None이면 stdout)
    convert::to_markdown(
        "report.hwpx".as_ref(),
        Some("report.md".as_ref()),
        Some("assets/".as_ref()),
        true, // YAML 프론트매터 출력
    )?;

    // Markdown → HWPX
    convert::to_hwpx(
        "draft.md".as_ref(),
        Some("draft.hwpx".as_ref()),
        None, // 스타일 템플릿
    )?;

    Ok(())
}
```

### 중간 표현(IR) 직접 사용

```rust
use hwp2md::{hwp, hwpx, md, ir};

// 문서를 IR로 파싱
let doc: ir::Document = hwpx::read_hwpx("report.hwpx".as_ref())?;

// 메타데이터 확인
if let Some(title) = &doc.metadata.title {
    println!("제목: {title}");
}

// 첫 번째 섹션의 블록 순회
for block in &doc.sections[0].blocks {
    if let ir::Block::Heading { level, inlines } = block {
        let text: String = inlines.iter().map(|i| i.text.as_str()).collect();
        println!("H{level}: {text}");
    } else if let ir::Block::Table { rows, col_count } = block {
        println!("표 {col_count}열 x {}행", rows.len());
    }
}

// Markdown으로 렌더링
let markdown = md::write_markdown(&doc, false);
println!("{markdown}");
```

### Markdown을 IR로 파싱

```rust
use hwp2md::md;

let source = std::fs::read_to_string("document.md")?;
let doc = md::parse_markdown(&source);
println!("{}개 섹션", doc.sections.len());
```

## 형식 지원 매트릭스

| 기능 | HWP 5.0 → MD | HWPX → MD | MD → HWPX |
|------|:---:|:---:|:---:|
| 제목 (H1-H6) | O | O | O |
| 문단 | O | O | O |
| 굵게 / 기울임 | O | O | O |
| 밑줄 | O | O | O |
| 취소선 | O | O | O |
| 인라인 코드 | O | O | O |
| 위첨자 / 아래첨자 | O | O | O |
| 하이퍼링크 | O | O | O |
| 루비 주석 | O | O | O |
| 순서 목록 | O | O | O |
| 비순서 목록 | O | O | O |
| 중첩 목록 | O | O | O |
| 표 | O | O | O |
| 이미지 (추출) | O | O | O |
| 코드 블록 | O | O | O |
| 인용 블록 | O | O | O |
| 각주 | O | O | O |
| 수식 (LaTeX) | O | O | O |
| YAML 프론트매터 | O | O | - |
| 다단 레이아웃 | 단일화 | 단일화 | - |
| 머리글 / 바닥글 | 미지원 | O | O |
| DRM 보호 HWP | X | X | - |
| MD → HWP 바이너리 | - | - | X |

## 아키텍처

```
HWP 5.0 (.hwp)  ──── hwp::read_hwp()   ──┐
                                         ├──> ir::Document ──> md::write_markdown() ──> Markdown
HWPX (.hwpx)    ──── hwpx::read_hwpx() ──┘
                                           ┌── ir::Document <── md::parse_markdown() <── Markdown
                                           └──> hwpx::write_hwpx() ──> HWPX (.hwpx)
```

변환 파이프라인은 형식 중립적인 중간 표현(`ir::Document`)으로 분리되어 있습니다. 모든 리더는 `ir::Document`를 생산하고, 모든 라이터는 이를 소비합니다. 형식별 코드가 격리되어 새로운 입출력 형식을 추가하기 쉽습니다.

### IR 핵심 타입

| 타입 | 설명 |
|------|------|
| `Document` | 루트: 메타데이터 + 섹션 + 에셋 |
| `Metadata` | 제목, 저자, 생성/수정일, 주제, 키워드 |
| `Section` | `Block` 값의 순서열 |
| `Block` | `Heading`, `Paragraph`, `Table`, `CodeBlock`, `BlockQuote`, `List`, `Image`, `HorizontalRule`, `Footnote`, `Math` |
| `Inline` | 스타일 플래그를 가진 텍스트 (굵게, 기울임, 밑줄, 취소선, 코드, 위/아래첨자, 링크, 각주, 루비) |
| `Asset` | 내장 바이너리 (이미지 등) + MIME 타입 |

### 크레이트 구조

```
src/
  main.rs          CLI 진입점 (clap)
  lib.rs           퍼블릭 re-export
  convert.rs       고수준 API: to_markdown / to_hwpx / show_info
  ir.rs            중간 표현 타입
  error.rs         Hwp2MdError 열거형 (thiserror)
  hwp/             HWP 5.0 리더 (CFB 컨테이너, 레코드 파서, EqEdit)
  hwpx/            HWPX 리더 + 라이터 (ZIP + quick-xml)
  md/              Markdown 파서 (comrak) + 라이터
tests/             통합 테스트
```

## 알려진 제한 사항

- DRM 보호(배포용) HWP 파일은 지원하지 않습니다.
- 다단 레이아웃은 단일 컬럼으로 평탄화됩니다.
- colspan/rowspan이 복잡한 표는 Markdown에서 HTML로 폴백합니다.
- HWP 5.0 바이너리의 머리글/바닥글은 건너뜁니다 (HWPX는 완전 지원).
- 바이너리 HWP 5.0 형식으로의 역변환(MD → HWP)은 미지원입니다. HWPX 출력만 가능합니다.

## 변환 버그 신고

`hwp2md`가 HWP/HWPX 파일을 올바르게 변환하지 못하는 경우, **[Conversion Bug](https://github.com/hephaex/hwp2md/issues/new?template=conversion-bug.yml)** 템플릿으로 이슈를 등록해 주세요. 문제가 되는 파일을 첨부하면 CI가 자동으로 변환을 시도하고 결과를 이슈 댓글로 게시합니다.

## 기여

버그 리포트와 풀 리퀘스트를 환영합니다: <https://github.com/hephaex/hwp2md>

패치 제출 전:

1. `cargo fmt` 및 `cargo clippy -- -D warnings` 실행
2. `cargo test --all-targets` 통과 확인
3. 변경된 동작에 대한 테스트 추가 또는 업데이트

## 라이선스

Copyright (c) 2026 Mario Cho \<hephaex@gmail.com\>

이 프로그램은 자유 소프트웨어입니다. GNU General Public License v3 (only) 조건 하에 재배포 및 수정할 수 있습니다.

전문은 [LICENSE](LICENSE)를 참조하세요.

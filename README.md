# hwp2md

HWP/HWPX ↔ Markdown 양방향 변환기 (Rust)

한글(HWP/HWPX) 문서를 Markdown으로, Markdown을 HWPX 문서로 변환합니다.

## Features

- **HWP → Markdown**: HWP 5.0 바이너리 문서를 Markdown으로 변환
- **HWPX → Markdown**: HWPX (XML 기반) 문서를 Markdown으로 변환
- **Markdown → HWPX**: Markdown 문서를 HWPX 파일로 변환
- **이미지 추출**: 문서 내 이미지를 별도 디렉토리로 추출
- **수식 변환**: HWP 수식을 LaTeX 문법으로 변환
- **스타일 템플릿**: YAML 기반 커스텀 스타일 지원 (MD→HWPX)

## Installation

```bash
cargo install hwp2md
```

또는 소스에서 빌드:

```bash
git clone https://github.com/hephaex/hwp2md.git
cd hwp2md
cargo build --release
```

## Usage

### HWP/HWPX → Markdown

```bash
# HWP 파일을 Markdown으로 변환
hwp2md to-md document.hwp -o output.md

# HWPX 파일을 Markdown으로 변환 (이미지 추출 포함)
hwp2md to-md document.hwpx -o output.md --assets-dir ./images

# 메타데이터를 YAML frontmatter로 포함
hwp2md to-md document.hwp -o output.md --frontmatter
```

### Markdown → HWPX

```bash
# Markdown을 HWPX로 변환
hwp2md to-hwpx document.md -o output.hwpx

# 커스텀 스타일 템플릿 적용
hwp2md to-hwpx document.md -o output.hwpx --style template.yaml
```

### 문서 정보

```bash
hwp2md info document.hwp
```

## Conversion Mapping

| HWP/HWPX | Markdown |
|-----------|----------|
| 제목 (개요 1~6) | `# ~ ######` |
| 굵게/기울임 | `**bold**` / `*italic*` |
| 취소선 | `~~text~~` |
| 하이퍼링크 | `[text](url)` |
| 표 | GFM table |
| 이미지 | `![alt](path)` |
| 코드 블록 | ` ```lang ``` ` |
| 인용 | `> text` |
| 목록 | `1.` / `-` |
| 각주 | `[^1]` |
| 수식 | `$LaTeX$` |

## Limitations

- HWP DRM (배포용) 문서는 지원하지 않습니다
- 다단 레이아웃은 단일 단으로 평탄화됩니다
- 복잡한 테이블 (colspan/rowspan)은 HTML 폴백을 사용합니다
- 머리글/바닥글은 변환에서 제외됩니다
- MD→HWP (바이너리)는 지원하지 않습니다 — HWPX만 출력

## Dependencies

이 프로젝트는 다음 Rust 크레이트를 활용합니다:

- [unhwp](https://github.com/iyulab/unhwp) — HWP/HWPX 파싱 + Markdown 추출
- [hwpforge](https://docs.rs/hwpforge/) — HWPX 프로그래밍 제어
- [comrak](https://github.com/kivikakk/comrak) — GFM Markdown 파싱
- [clap](https://github.com/clap-rs/clap) — CLI 인터페이스

## License

GPL-3.0-only

Copyright (c) 2026 Mario Cho <hephaex@gmail.com>

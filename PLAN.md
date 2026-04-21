# hwp2md — Implementation Plan

HWP/HWPX ↔ Markdown 양방향 변환기 (Rust)

## 목표

한글(HWP/HWPX) 문서를 Markdown으로, Markdown을 HWPX 문서로 변환하는 CLI 도구 + 라이브러리.
모든 HWP 파싱/생성 코드를 자체 구현 — 라이선스 독립성 확보.

## 설계 원칙

1. **라이선스 독립**: HWP 전용 크레이트(unhwp, hwpforge, hwpers 등) 미사용
2. **Generic 의존성만 사용**: cfb, zip, quick-xml, flate2, comrak 등 범용 크레이트
3. **IR 기반 아키텍처**: 포맷별 reader/writer가 공통 IR을 통해 변환
4. **점진적 구현**: 텍스트 추출 → 서식 → 테이블 → 이미지 → 수식 순

## 의존성

```toml
# 컨테이너/압축 (범용)
cfb = "0.14"           # OLE2/CFB — HWP 5.0 컨테이너
zip = "2.0"            # ZIP — HWPX 컨테이너
flate2 = "1.0"         # Zlib — HWP 스트림 압축

# 파싱 (범용)
quick-xml = "0.37"     # XML — HWPX 콘텐츠
byteorder = "1.5"      # 바이트 오더 — HWP 레코드
encoding_rs = "0.8"    # 문자 인코딩

# Markdown (범용)
comrak = "0.34"        # GFM Markdown 파서/렌더러

# CLI/유틸 (범용)
clap = "4"
serde = "1"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
```

## 아키텍처

```
HWP 5.0 ──→ hwp::reader (CFB+zlib+record) ──→ IR ──→ md::writer ──→ Markdown
HWPX    ──→ hwpx::reader (ZIP+XML)        ──→ IR ──→ md::writer ──→ Markdown

Markdown ──→ md::parser (comrak)           ──→ IR ──→ hwpx::writer ──→ HWPX
```

### HWP 5.0 Record Format

```
┌─────────────────────────────────────────┐
│ 4-byte Header (Little Endian)           │
│   bits  0-9:  tag_id (0x3FF mask)       │
│   bits 10-19: level   (0x3FF mask)      │
│   bits 20-31: size    (0xFFF mask)      │
│   if size == 0xFFF → 4-byte extended    │
├─────────────────────────────────────────┤
│ data[size]                              │
└─────────────────────────────────────────┘
```

Text: UTF-16LE, control chars 0x0000-0x001F (extended controls use 8 code units)

### 중간 표현 (IR)

```
Document
├── Metadata (title, author, dates, keywords)
├── Sections[]
│   └── Blocks[]
│       ├── Heading (level, inlines)
│       ├── Paragraph (inlines)
│       ├── Table (rows → cells, col_count)
│       ├── CodeBlock (language, code)
│       ├── BlockQuote (nested blocks)
│       ├── List (ordered, start, items)
│       ├── Image (src, alt)
│       ├── HorizontalRule
│       ├── Footnote (id, content)
│       └── Math (display, tex)
└── Assets[] (name, data, mime_type)
```

### 모듈 구조

```
src/
├── main.rs          — CLI (to-md, to-hwpx, info)
├── lib.rs           — 모듈 선언
├── convert.rs       — 변환 오케스트레이터
├── error.rs         — 에러 타입
├── ir.rs            — 중간 표현
├── hwp/
│   ├── mod.rs       — HWP 모듈 공개 인터페이스
│   ├── model.rs     — HWP 내부 모델 (FileHeader, DocInfo, Section, Paragraph)
│   ├── record.rs    — 레코드 파싱 (4-byte header, tag constants)
│   └── reader.rs    — CFB → zlib → records → IR 변환
├── hwpx/
│   ├── mod.rs       — HWPX 모듈 공개 인터페이스
│   ├── reader.rs    — ZIP+XML → IR 변환
│   └── writer.rs    — IR → HWPX (ZIP+XML) 생성
└── md/
    ├── mod.rs       — Markdown 모듈 공개 인터페이스
    ├── writer.rs    — IR → Markdown 렌더러
    └── parser.rs    — Markdown → IR 파서 (comrak)
```

## 구현 단계

### Phase 1: 프로젝트 구조 + 기본 파싱 (완료)

자체 구현 기반 프로젝트 재구성.

- [x] 1.1 Cargo.toml — HWP 전용 크레이트 제거, generic 의존성만
- [x] 1.2 IR 설계 — Document/Section/Block/Inline 계층 + Asset
- [x] 1.3 HWP reader — CFB + zlib + record parsing + UTF-16LE text
- [x] 1.4 HWPX reader — ZIP + XML 파싱
- [x] 1.5 MD writer — IR → Markdown (GFM 호환)
- [x] 1.6 MD parser — Markdown → IR (comrak 기반)
- [x] 1.7 HWPX writer — IR → HWPX (ZIP+XML 생성)
- [x] 1.8 Convert 오케스트레이터 — to-md, to-hwpx, info 명령

### Phase 2: HWP 파싱 고도화 (1주)

HWP 5.0 파서 정확도 향상.

- [ ] 2.1 테이블 파싱 — CTRL_TABLE + LIST_HEADER + 셀 내 paragraphs
- [ ] 2.2 이미지/바이너리 — BinData 참조 + 이미지 추출
- [ ] 2.3 하이퍼링크 — CTRL 오브젝트에서 URL 추출
- [ ] 2.4 각주/미주 — CTRL_FOOTNOTE/CTRL_ENDNOTE 파싱
- [ ] 2.5 수식 — EQEDIT 스크립트 → LaTeX 변환
- [ ] 2.6 DRM/암호화 감지 — 명확한 에러 메시지
- [ ] 2.7 샘플 테스트 — 다양한 HWP 파일로 검증

### Phase 3: HWPX 파싱 고도화 (1주)

HWPX XML 파서 정확도 향상.

- [ ] 3.1 스타일 상속 — header.xml 스타일 정의 파싱
- [ ] 3.2 테이블 — colspan/rowspan 처리
- [ ] 3.3 이미지 — BinData 참조 해결
- [ ] 3.4 각주/미주 — XML 요소 매핑
- [ ] 3.5 수식 — OWPML 수식 요소 → LaTeX
- [ ] 3.6 정부 공문서 HWPX 샘플 검증

### Phase 4: Markdown 렌더러 고도화 (1주)

- [ ] 4.1 GFM 호환 검증 — comrak 왕복 테스트
- [ ] 4.2 복잡한 테이블 — colspan/rowspan → HTML 폴백
- [ ] 4.3 이미지 참조 — 상대경로 / inline base64 옵션
- [ ] 4.4 frontmatter — YAML 메타데이터
- [ ] 4.5 info 명령 — 문서 통계 (페이지, 글자수, 스타일)

### Phase 5: HWPX 라이터 고도화 (1주)

- [ ] 5.1 스타일 매핑 — heading/paragraph 스타일 정의
- [ ] 5.2 테이블 생성 — Markdown 테이블 → HWPX 표
- [ ] 5.3 이미지 삽입 — Asset → HWPX BinData
- [ ] 5.4 YAML 스타일 템플릿 — 커스텀 스타일 지원
- [ ] 5.5 한글 2022+ 호환 검증
- [ ] 5.6 라운드트립 테스트 — HWP→MD→HWPX 왕복 검증

### Phase 6: CLI 완성 + 배포 (1주)

- [ ] 6.1 에러 처리 — 사용자 친화적 에러 메시지
- [ ] 6.2 배치 변환 — 디렉토리 일괄 변환
- [ ] 6.3 CI/CD — GitHub Actions (build + test + clippy)
- [ ] 6.4 테스트 커버리지 80%+
- [ ] 6.5 crates.io 배포
- [ ] 6.6 README 업데이트

## 변환 매핑 테이블

| HWP/HWPX 요소 | Markdown 매핑 | 비고 |
|----------------|---------------|------|
| 제목 (개요 1~6) | `# ~ ######` | 수준 매핑 |
| 본문 | 일반 텍스트 | |
| 굵게/기울임 | `**bold**` / `*italic*` | |
| 밑줄 | `<u>text</u>` | HTML 폴백 |
| 취소선 | `~~text~~` | GFM |
| 위첨자/아래첨자 | `<sup>/<sub>` | HTML 폴백 |
| 하이퍼링크 | `[text](url)` | |
| 표 | GFM table | colspan → HTML 폴백 |
| 이미지 | `![alt](path)` | 파일 추출 |
| 코드 블록 | ` ```lang ``` ` | 고정폭 스타일 추론 |
| 인용 | `> text` | |
| 순서/비순서 목록 | `1.` / `-` | 중첩 지원 |
| 각주 | `[^1]` | GFM footnotes |
| 수식 | `$LaTeX$` | 한글 수식 → LaTeX |
| 머리글/바닥글 | 무시 또는 frontmatter | |
| 다단 | 단일 단으로 평탄화 | |

## 제한사항

- HWP DRM (배포용) 문서는 지원하지 않음
- 다단 레이아웃은 단일 단으로 평탄화
- 복잡한 테이블 (colspan/rowspan)은 HTML 폴백
- 머리글/바닥글은 변환에서 제외
- MD → HWP (바이너리)는 지원하지 않음 — HWPX만 출력
- 한글 수식 → LaTeX 변환은 기본적인 수준만 지원

## 라이선스

GPL-3.0-only

## 참조

- [HWP 5.0 스펙](https://www.hancom.com/support/downloadCenter/hwpOwpml) — 한컴 공식
- [OWPML (KS X 6101)](https://tech.hancom.com/hwpxformat/) — HWPX 포맷 설명
- [rhwp](https://github.com/edwardkim/rhwp) — Rust HWP 뷰어/에디터 (참조용)
- [hwpforge](https://docs.rs/hwpforge/) — HWPX 프로그래밍 제어 (참조용)
- [hwpConverter](https://github.com/vsdn/hwpConverter) — Java HWP↔HWPX 변환 (참조용)

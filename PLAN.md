# hwp2md — Implementation Plan

HWP/HWPX ↔ Markdown 양방향 변환기 (Rust)

## 목표

한글(HWP/HWPX) 문서를 Markdown으로, Markdown을 HWPX 문서로 변환하는 CLI 도구 + 라이브러리.

## 기술 조사 결과

### 기존 Rust 크레이트 현황

| 크레이트 | 버전 | 용도 | 비고 |
|----------|------|------|------|
| `unhwp` | 0.2.4 | HWP/HWPX → Markdown 추출 | HWP5/HWPX/HWP3 지원, 이미지 추출, LaTeX 수식 변환 |
| `hwpforge` | 0.5.1 | HWPX 프로그래밍 제어 (read/write) | HWPX만 지원 (HWP binary 미지원) |
| `hwpforge-smithy-md` | 0.5.1 | Markdown ↔ hwpforge IR 코덱 | 양방향 변환 가능 |
| `hwpforge-smithy-hwpx` | 0.5.1 | HWPX ↔ hwpforge IR 코덱 | encode + decode |
| `hwpers` | 0.5.0 | HWP 5.0 파싱 + 레이아웃 렌더링 | 190★, 가장 성숙한 HWP Rust 파서 |
| `hwp` | 0.2.0 | 저수준 HWP 파서 | 기본적 |
| `hwarang` | 0.2.0 | HWP 텍스트 추출기 | 텍스트만 |
| `jw-hwp-core` | 0.1.1 | HWP5/HWPX 읽기 전용 파서 | MCP 서버 포함 |

### 기존 도구 (타 언어)

| 도구 | 언어 | 용도 |
|------|------|------|
| `hwp.js` | TypeScript | HWP 뷰어/파서, 1.3K★ |
| `pyhwp` | Python | HWP5 파서, ODT/TXT 변환, 296★ |
| `hwplib` | Java | HWP read/write, Maven 배포 |
| `rhwp` | Rust+WASM | HWP/HWPX 뷰어/에디터, 1.8K★ |
| `md2hml` | Python | Markdown → HWP (HML 경유), 70★ |
| `vsdn/hwpConverter` | Java | HWPX↔HWP 양방향 + DRM |

### HWP 포맷 구조

**HWP 5.0 (바이너리)**:
- OLE2/CFB 컨테이너 (Microsoft Compound File Binary)
- Zlib 압축 스트림 (FileHeader, DocInfo, BodyText/, BinData/)
- 레코드 기반 구조 (태그 ID + 크기 + 데이터)
- UTF-16LE 문자 인코딩
- 공식 스펙: "한글문서파일형식 5.0 revision 1.3" (한컴 공개)

**HWPX (XML 기반)**:
- ZIP 아카이브 컨테이너
- XML 콘텐츠 (OWPML — KS X 6101 국가표준)
- HWP 2010부터 지원, 2021년 공공문서 표준
- DOCX와 유사한 구조 (파싱이 훨씬 쉬움)

### 핵심 의존성

```toml
# HWP→MD (Phase 1-3): unhwp이 핵심 — 이미 HWP/HWPX→MD 변환 구현됨
unhwp = "0.2"

# MD→HWPX (Phase 4-5): hwpforge가 핵심 — MD↔HWPX 양방향 코덱
hwpforge = "0.5"
hwpforge-smithy-md = "0.5"
hwpforge-smithy-hwpx = "0.5"

# 저수준 보조 (커스텀 처리 필요 시)
cfb = "0.14"          # OLE2 컨테이너
flate2 = "1.0"        # Zlib 압축
quick-xml = "0.37"    # XML 파싱
comrak = "0.34"       # GFM Markdown 파싱
```

## 아키텍처

```
HWP 5.0 ──→ unhwp ──→ IR (Document) ──→ md_writer ──→ Markdown
HWPX    ──→ unhwp ──→ IR (Document) ──→ md_writer ──→ Markdown

Markdown ──→ md_parser ──→ IR (Document) ──→ hwpforge ──→ HWPX
```

### 중간 표현 (IR)

```
Document
├── Metadata (title, author, dates)
├── Sections[]
│   └── Blocks[]
│       ├── Heading (level, inlines)
│       ├── Paragraph (inlines)
│       ├── Table (rows → cells)
│       ├── CodeBlock (language, code)
│       ├── BlockQuote (nested blocks)
│       ├── List (ordered/unordered, items)
│       ├── Image (src, alt)
│       ├── HorizontalRule
│       └── Footnote (id, content)
└── Assets[] (embedded images/binaries)
```

## 구현 단계

### Phase 1: HWP 5.0 → Markdown (2주)

unhwp 크레이트를 래핑하여 HWP 바이너리 → Markdown 변환 구현.

- [ ] 1.1 unhwp 통합 — `parse_file()` + `render_markdown()` 래핑
- [ ] 1.2 IR 변환 — unhwp 출력을 내부 IR로 매핑
- [ ] 1.3 이미지 추출 — assets 디렉토리로 이미지 저장
- [ ] 1.4 메타데이터 — frontmatter (YAML) 출력
- [ ] 1.5 테스트 — 샘플 HWP 파일 5개 이상으로 검증

**산출물**: `hwp2md to-md input.hwp -o output.md --assets-dir ./images`

### Phase 2: HWPX → Markdown (1주)

unhwp의 HWPX 지원을 활용. HWPX는 XML 기반이라 HWP보다 정확도 높음.

- [ ] 2.1 HWPX 파싱 — unhwp의 HWPX 경로 활성화
- [ ] 2.2 테이블 처리 — GFM 테이블 포맷, colspan/rowspan 폴백
- [ ] 2.3 수식 변환 — 수식을 LaTeX ($...$) 또는 이미지로
- [ ] 2.4 각주/미주 — Markdown 각주 문법 [^1] 매핑
- [ ] 2.5 테스트 — 정부 공문서 HWPX 샘플 검증

**산출물**: `hwp2md to-md input.hwpx -o output.md`

### Phase 3: Markdown 렌더러 고도화 (1주)

IR → Markdown 변환 품질 개선.

- [ ] 3.1 GFM 호환 — comrak 기반 출력 검증
- [ ] 3.2 복잡한 테이블 — HTML 폴백 (merged cells)
- [ ] 3.3 이미지 참조 — 상대경로 / inline base64 옵션
- [ ] 3.4 문서 구조 보존 — heading hierarchy, 목차 생성
- [ ] 3.5 `info` 명령 — 문서 메타정보 표시 (페이지수, 글자수, 스타일 등)

### Phase 4: Markdown → IR 파서 (1주)

comrak/pulldown-cmark으로 Markdown을 IR로 파싱.

- [ ] 4.1 Markdown 파서 — comrak AST → IR 변환
- [ ] 4.2 인라인 서식 — bold/italic/code/link/strikethrough
- [ ] 4.3 블록 요소 — heading/paragraph/table/code/quote/list
- [ ] 4.4 이미지 임베딩 — 로컬 이미지 → Asset으로 수집
- [ ] 4.5 각주 파싱 — [^n] 문법 → IR Footnote

### Phase 5: IR → HWPX 라이터 (2주)

hwpforge를 활용하여 IR을 유효한 HWPX 파일로 출력.

- [ ] 5.1 hwpforge 통합 — IR → hwpforge Document 매핑
- [ ] 5.2 스타일 매핑 — heading/paragraph 스타일 정의
- [ ] 5.3 테이블 생성 — Markdown 테이블 → HWPX 표
- [ ] 5.4 이미지 삽입 — Asset → HWPX BinData
- [ ] 5.5 스타일 템플릿 — YAML 기반 커스텀 스타일 (hwpforge-blueprint)
- [ ] 5.6 출력 검증 — 한글 2022+에서 열기 테스트
- [ ] 5.7 라운드트립 테스트 — HWP→MD→HWPX 왕복 검증

**산출물**: `hwp2md to-hwpx input.md -o output.hwpx --style template.yaml`

### Phase 6: CLI 완성 + 배포 (1주)

- [ ] 6.1 에러 처리 — 사용자 친화적 에러 메시지
- [ ] 6.2 로깅 — tracing 기반 상세 로그
- [ ] 6.3 배치 변환 — 디렉토리 일괄 변환 지원
- [ ] 6.4 CI/CD — GitHub Actions (build + test + clippy)
- [ ] 6.5 crates.io 배포 — `cargo publish`
- [ ] 6.6 README 업데이트 — 사용법, 예시, 제한사항

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
| 코드 블록 | ` ```lang ``` ` | 스타일 추론 |
| 인용 | `> text` | |
| 순서/비순서 목록 | `1.` / `-` | 중첩 지원 |
| 각주 | `[^1]` | kramdown/GFM |
| 수식 | `$LaTeX$` | unhwp LaTeX 변환 |
| 머리글/바닥글 | 무시 또는 frontmatter | |
| 다단 | 단일 단으로 평탄화 | 구조 손실 |
| 글상자 | blockquote 또는 무시 | |

## 위험 요소

| 위험 | 영향 | 대응 |
|------|------|------|
| unhwp가 특정 HWP 변형 미지원 | 파싱 실패 | hwpers 폴백 또는 직접 파싱 |
| hwpforge HWPX 출력이 한글에서 안 열림 | MD→HWPX 불가 | 한글 2022+ 직접 검증, 최소 구조 테스트 |
| colspan/rowspan 표 → Markdown 손실 | 포맷 열화 | HTML 테이블 폴백 |
| 수식 변환 정확도 | LaTeX 깨짐 | 이미지 폴백 옵션 |
| HWP DRM (배포용 문서) | 읽기 불가 | 에러 메시지로 안내 |

## 라이선스

GPL-3.0-only

## 참조

- [HWP 5.0 스펙](https://www.hancom.com/support/downloadCenter/hwpOwpml) — 한컴 공식
- [OWPML (KS X 6101)](https://tech.hancom.com/hwpxformat/) — HWPX 포맷 설명
- [unhwp](https://github.com/iyulab/unhwp) — HWP/HWPX → Markdown Rust 라이브러리
- [hwpforge](https://docs.rs/hwpforge/) — HWPX 프로그래밍 제어
- [rhwp](https://github.com/edwardkim/rhwp) — Rust HWP 뷰어/에디터
- [hwpConverter](https://github.com/vsdn/hwpConverter) — Java HWP↔HWPX 변환
- [md2hml](https://github.com/msjang/md2hml) — Python Markdown → HWP
- [pyhwp](https://github.com/mete0r/pyhwp) — Python HWP 파서

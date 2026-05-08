# hwp2md — Progress

## 현재 상태: v0.5.0 Sprint 25 완료 (map_or + clone_from + char patterns)

### 완료

- [x] 기술 조사 — 기존 Rust 크레이트 현황 파악 (unhwp, hwpforge, hwpers 등)
- [x] 기존 도구 조사 — 타 언어 HWP 변환 도구 전수 조사
- [x] HWP/HWPX 포맷 구조 분석
- [x] 아키텍처 재설계 — 자체 구현, HWP 전용 크레이트 제거
- [x] Phase 1 전체 완료:
  - HWP 5.0 reader (CFB + zlib + record parsing + UTF-16LE text + char shape)
  - HWPX reader (ZIP + XML + section parsing + table/image/equation)
  - IR 설계 (Document/Section/Block/Inline + Math + Footnote + Asset)
  - Markdown writer (GFM 호환, frontmatter, table, HTML fallback)
  - Markdown parser (comrak 기반, footnote, math, table)
  - HWPX writer (ZIP + XML, OWPML 구조)
  - Convert 오케스트레이터 (to-md, to-hwpx, info)
  - cargo check 통과
- [x] 보안/정확도 수정 (99a1bc6):
  - C1-C6: 압축 폭탄, 레코드 할당, 경계 검사, 서로게이트, 경로 순회, ZIP-slip
  - H1: 압축 해제 에러 로깅
  - H6/H7: 이미지 alt text 수정
  - H9: 멀티바이트 글자수 계산 수정
  - clippy 경고 수정 (derive Default, range contains, collapsible if)
- [x] Phase 1.5 (d877719 + f4d4564):
  - 103 테스트 작성 (89 unit + 14 integration, 0 failures)
  - H2: CharShape 속성 오프셋 46-49로 수정 (was 60-63)
  - H3: 제목 감지 임계값 상수화 (16pt/14pt/12pt)
  - H5: charPr OWPML 위치 확인 + 문서화
  - H8: 프론트매터 keywords YAML 파싱
  - M3: HWPX writer Result 전파 (`?` 사용)
  - M6: YAML escape \n/\r/\t 추가
  - M8: parse_heading_style 끝 숫자 추출
  - M10: encoding_rs, serde_yaml 미사용 의존성 제거
  - 리뷰 수정: keyword escape, height clamp, 인라인 서식 라운드트립 테스트
- [x] Phase 2 (14694df + 77db271):
  - HWP 제어 문자 파싱: 테이블(CTRL_TABLE+LIST_HEADER), 이미지(GSHAPE+GSOTYPE), 각주/미주
  - Level-aware subtree 탐색 (find_children_end)
  - read_section_stream 인덱스 기반 루프 리팩토링
  - M7: OLE2 SummaryInformation 메타데이터 추출 (title/author/subject/keywords)
  - M9: BIN 스트림 doc_info.bin_data_entries 기반 최적화, fallback 100 cap
  - H4: --style 파라미터 tracing::warn 경고
  - 리뷰 수정: checked_mul 오버플로우 가드, 행 인덱스 10K cap, gshape/gsotype 테스트 5건
  - 130 테스트 (116 unit + 14 integration, 0 failures)

- [x] Phase 2b + 아키텍처 리팩토링 (71e54ea):
  - reader.rs 서브모듈 분할: reader(828L), control(781L), convert(434L), summary(238L)
  - 하이퍼링크 URL 추출 (CTRL_HYPERLINK + parse_hyperlink_url + IR 변환)
  - parse_summary_bytes 분리 + OLE2 바이트 버퍼 테스트 4건
  - DRM/암호화 감지 (has_drm 비트 검사 + Hwp2MdError)
  - 142 테스트 (128 unit + 14 integration, 0 failures)

- [x] Phase 2c: EQEDIT→LaTeX + CI (37d2645 + da92270):
  - eqedit.rs: 토크나이저 + 멀티패스 변환기 (분수/그리스문자/연산자/매트릭스/파일/루트/구분자)
  - convert.rs에서 Equation control → eqedit_to_latex → IR Math 블록
  - MAX_RECURSION_DEPTH=32 스택 오버플로우 방지
  - GitHub Actions CI: cargo check + clippy + fmt + test (Rust 1.75.0 MSRV)
  - 42 EQEDIT 테스트 추가
  - 187 테스트 (173 unit + 14 integration, 0 failures)

- [x] Phase 2d: HWPX 테스트 + Dead code 정리 (5e297d7):
  - hwpx/reader.rs: 42 단위 테스트 추가 (parse_section_xml, guess_mime_from_name)
    - 단락, 제목, charPr(bold/italic/underline/strikeout), 테이블(colspan/rowspan),
      이미지, 수식, 목록, 줄바꿈, MIME 타입 (13종)
  - record.rs: 6개 dead code 경고 수정 (per-item #[allow(dead_code)])
  - CI 호환: clippy -D warnings 0 경고
  - 229 테스트 (215 unit + 14 integration, 0 failures)

- [x] Phase 3: HWPX 고도화 (bf57712):
  - charPr 리팩토링: apply_charpr_attrs() 헬퍼로 Start/Empty 중복 제거
  - `<li>/<hp:li>` 핸들러: ParseContext에 list_item 컨텍스트 추가, ListItem 생성
  - 각주/미주 파싱: `<hp:fn>`, `<hp:en>`, `<hp:footnote>`, `<hp:endnote>` → IR Footnote
  - 각주 참조: `<hp:noteRef>`, `<hp:ctrl id="fn">` → inline footnote_ref
  - BinData 참조 해결: build_bin_map + resolve_bin_refs (재귀, 모든 컨테이너 블록)
  - 리뷰 수정 4건 (HIGH): Footnote/List/BlockQuote 재귀, lineBreak/img/noteRef 컨텍스트 라우팅
  - 교차 테스트 5건 (image-in-footnote, image-in-list, linebreak-in-list, resolve-in-footnote/list)
  - 243 테스트 (229 unit + 14 integration, 0 failures)

- [x] Phase 3b: 모듈 분할 + 테스트 확충 (35ea51f):
  - reader.rs 분할: 1895→931행 (테스트 968행 → reader_tests.rs 분리)
  - ParseContext 디스패치 메서드: active_text_buf(), push_inline(), push_block_scoped()
  - 5+ 핸들러 라우팅 중복 제거 (handle_text, end:t, lineBreak, img, noteRef, ctrl)
  - writer.rs 테스트 31건 (generate_section_xml 22건 + write_hwpx ZIP 9건)
  - convert.rs 테스트 39건 (count_chars 21건 + write_assets 3건 + orchestrator 4건)
  - count_chars 버그 수정: `_ => 0` → exhaustive match (Footnote 재귀 누락)
  - count_chars: .len() → .chars().count() (CJK 안전)
  - 리뷰 수정 2건 (HIGH): 디스패치 우선순위 통일 문서화, chars().count() 일관성
  - 302 테스트 (288 unit + 14 integration, 0 failures)

- [x] Phase 4: Writer 고도화 + MD 안전성 (c9780c5):
  - HWPX writer: `<hp:cellAddr colSpan rowSpan/>` 출력 (span > 1일 때)
  - HWPX writer: 테스트 분리 → writer_tests.rs (814→279행)
  - MD writer: escape_inline() 추가 (7종 GFM 메타문자 이스케이프)
  - MD writer: 빈 텍스트 formatting marker 방지 (`****` 버그)
  - MD writer: cell_to_text exhaustive match (catch-all 제거)
  - MD writer: 테스트 분리 → writer_tests.rs (876→310행)
  - Block enum match 전수 감사: 코드베이스 전체 catch-all 0건
  - 18 인라인 엣지케이스 + 4 cellAddr 테스트
  - 320 테스트 (306 unit + 14 integration, 0 failures)

- [x] Phase 5: Parser 고도화 + 테스트 커버리지 (8360c8f):
  - MD parser: InlineStyle에 underline/subscript 추가 + HtmlInline 상태머신
  - `<u>`/`</u>`, `<sub>`/`</sub>` 태그 → IR underline/subscript 플래그
  - close-tag restoration: 부모 스타일 복원 (중첩 안전)
  - 16 parser 테스트 (frontmatter, footnote, math, image, 인라인 스타일)
  - 11 CLI 통합 테스트 (help, version, 에러 처리, end-to-end HWPX↔MD)
  - 12 roundtrip 테스트 (math, footnote, image, frontmatter, Korean, escaped text)
  - 359 테스트 (322 unit + 11 CLI + 26 roundtrip, 0 failures)

- [x] Phase 6: 커버리지 + crates.io 준비 + 엣지케이스 (11d91c0):
  - cargo-tarpaulin 커버리지: 75.64% → 82.28% (80%+ 달성)
  - 100 unit 테스트 추가 (record, model, convert, summary, writer, parser)
  - README.md 재작성 (배지, 설치, CLI/라이브러리 사용법, 아키텍처)
  - Cargo.toml: homepage, documentation, readme 필드 추가
  - escape_paragraph_line_start: 멀티라인 안전, #/>/-/*/+/1./--- 커버
  - detect_heading_level: H7 → H6 clamp (.min(6))
  - text.len() → text.chars().count() (CJK heading 감지 정확도)
  - parser.rs, convert.rs 테스트 분리 (800행 제약 준수)
  - nested `<u><sub>` 파서 테스트, roundtrip code_block 2-pass 테스트
  - 리뷰 수정 4건 (HIGH): 멀티라인 이스케이프, H7 clamp, 파일 분할, byte→chars
  - 475 테스트 (437 unit + 11 CLI + 27 roundtrip, 0 failures)

- [x] Phase 7: 모듈 분할 + 이미지 임베딩 + 배포 준비 (f10633c):
  - hwpx/reader.rs 분할: ParseContext → context.rs (932→696행)
  - hwp/reader.rs 분할: shape parsers → shapes.rs (827→703행)
  - HWPX writer: BinData 이미지 임베딩 (ZIP 출력에 바이너리 포함)
  - Cargo.toml: exclude 필드 추가, cargo publish --dry-run 통과 (93.8 KiB)
  - no-assert 테스트 assertion 강화 (eqedit, parser soft_break)
  - 모든 프로덕션 파일 800행 이하 달성
  - 478 테스트 (440 unit + 11 CLI + 27 roundtrip, 0 failures)

- [x] Phase 8: 테스트 분리 + 통합 테스트 인프라 + 토크나이저 버그 수정 (ac63b6f):
  - eqedit.rs 테스트 분리: 42 테스트 → eqedit_tests.rs (829→563행)
  - HwpxFixture 빌더: 프로그래매틱 HWPX ZIP 생성 (tests/fixtures/mod.rs)
  - 24 통합 테스트: 빈 문서, 메타데이터, 단락, 제목, 테이블, 서식, 혼합, API, ZIP 구조, 엣지케이스
  - 토크나이저 무한 루프 수정: unmatched '}' 처리 누락 → tokenise()에 '}' 핸들러 추가
  - 2 신규 에지케이스 테스트: deep_nesting_does_not_panic, unmatched_closing_brace_no_underflow
  - cargo publish --dry-run 통과
  - 504 테스트 (442 unit + 11 CLI + 24 integration + 27 roundtrip, 0 failures)

- [x] Phase 9a: rhwp 비교 분석 + MUST 버그 수정 (d108d62):
  - rhwp (148K LOC) 대비 갭 분석: 파싱 정확도 중심
  - MUST fix: CharShape superscript/subscript 파싱 (attr bits 16-17)
  - MUST fix: ParaShape heading_type 파싱 (attr1 bits 24-25, outline level)
  - MUST fix: HWPX charPr superscript/subscript 속성 + flush 전파
  - 6 신규 테스트 (superscript/subscript/heading_type)
  - 510 테스트 (448 unit + 11 CLI + 24 integration + 27 roundtrip, 0 failures)

- [x] v0.5.0 Sprint 1 — A-1/A-2 + B-1 + B-2 + C-1 (139ab22):
  - Phase A-1: ParseContext 37필드 → 5 sub-structs 리팩토링
  - Phase A-2: PageLayout 파싱 50행 중복 제거
  - Phase B-1: YAML StyleTemplate (`--style`) — 페이지/폰트/행간 커스텀
  - Phase B-2: Task list 지원 (`- [x]` / `- [ ]`) — IR + parser + writer
  - Phase C-1: `--check` 서브커맨드 — 파일 유효성 검증 (출력 없이 파싱)
  - serde_yaml → serde_yml 0.0.12 마이그레이션
  - PageLayout `Copy` derive + StyleTemplate validation
  - 997 테스트 (868 unit + 17 CLI + 46 integration + 27 roundtrip + 기타, 0 failures)

- [x] v0.5.0 Sprint 2 — D-1 + check guard + test (c616de1):
  - Phase D-1: Criterion 벤치마크 5종 (MD→IR 107µs, IR→MD 30µs, IR→HWPX 527µs, HWPX→IR 139µs, roundtrip 781µs)
  - check() MD 파일 크기 가드 (MAX_MD_FILE_SIZE 256MB)
  - 순서 있는 task list 테스트 추가 (ordered_task_list_items)
  - 1001 테스트 (872 unit + 17 CLI + 46 integration + 27 roundtrip + 기타, 0 failures)

- [x] v0.5.0 Sprint 3 — B-3 + C-2 + M1 (2a12107):
  - Phase B-3: `Block::PageBreak` IR 변형 + HWPX `<hp:ctrl id="newPage|pageBreak|cnpb"/>` 인식 + `<hp:p><hp:run><hp:ctrl id="newPage"/></hp:run></hp:p>` 작성 + MD `<!-- pagebreak -->` 마커 양방향 지원
  - Phase C-2: `convert <input> <output>` 서브커맨드 — 확장자 기반 변환 방향 자동 감지 (`hwp/hwpx ↔ md/markdown`), 동일 포맷/미지원 조합은 명확한 에러
  - Sprint 2 리뷰 M1 수정: 전용 `Hwp2MdError::FileTooLarge { path, size, limit }` 변형으로 256MB 가드 분리
  - 1021 테스트 (887 unit + 21 CLI + 46 integration + 27 roundtrip + 기타, 0 failures)

- [x] v0.5.0 Sprint 4 — B-4 + M3 + C-3 (ad511aa):
  - Phase B-4: Header/footer 읽기 — `ir::Section`에 `header`/`footer` `Option<Vec<Block>>` 추가, HWPX `<hp:headerFooter>` → `<hp:header>`/`<hp:footer>` 파싱, MD `<!-- header -->` / `<!-- footer -->` 마커 출력, HWPX writer round-trip
  - Phase M3 (Sprint 3 리뷰): `convert` 서브커맨드 `--force` 플래그 — 기존 출력 파일 덮어쓰기 보호
  - Phase C-3: `ConvertOptions<'a>` builder 패턴 — `assets_dir`, `frontmatter`, `style`, `force` 통합 fluent API, `lib.rs`에서 re-export
  - 리뷰 수정: `push_block_scoped`/`flush_active_paragraph_scope`에 header/footer scope 분기 추가, `headerFooter` 시작 시 블록 버퍼 초기화
  - 1062 테스트 (923 unit + 23 CLI + 46 integration + 29 roundtrip + 기타, 0 failures)

- [x] v0.5.0 Sprint 5 — C-4 + D-2 + D-3 (이번 스프린트):
  - Phase C-4: 구조화된 에러 — `OutputExists { path }` (--force 가드), `DrmProtected { path }` (암호화 HWP) 전용 variant 추가, convert_auto/ConvertOptions/hwp::reader 마이그레이션
  - Phase D-2: Cross-platform CI — ubuntu/windows/macos 매트릭스, MSRV 1.75.0, lint 분리(ubuntu only)
  - Phase D-3: Coverage reporting — cargo-tarpaulin + Codecov 업로드 + README 배지
  - 리뷰 수정 (CRITICAL): `read_hwp()` lenient fallback이 DrmProtected 에러를 삼키는 버그 수정, taiki-e/install-action으로 tarpaulin 설치 최적화
  - 1065 테스트 (926 unit + 23 CLI + 46 integration + 29 roundtrip + 기타, 0 failures)

- [x] v0.5.0 Sprint 6 — MD header/footer round-trip + DRM integration test + headerFooter type (21a3399):
  - S6-01: MD parser region state machine — `<!-- header/footer -->` 마커를 comrak AST 수준에서 감지, body/header/footer 버킷 라우팅
  - S6-02: DrmProtected 통합 테스트 — `cfb::CompoundFile::create()`로 has_drm 비트 설정된 HWP fixture 생성 (Sprint 5 M2)
  - S6-03: `<hp:headerFooter type="both|even|odd">` 속성 파싱/출력 — IR Section에 `header_footer_type` 필드 추가 (Sprint 4 L1)
  - 1082 테스트 (940 unit + 23 CLI + 46 integration + 30 roundtrip + 기타, 0 failures)

- [x] v0.5.0 Sprint 7 — unclosed marker fallback + HeaderFooterType enum + ruby HTML parsing (e4ae1d9 + a098423):
  - S7-01: Unclosed marker fallback — `region != Body` at EOF → warn + drain all non-body blocks to body (Sprint 6 M1)
  - S7-02: `HeaderFooterType` enum (`Both`/`Even`/`Odd`/`Other(String)`) replacing `Option<String>` (Sprint 6 M3)
  - S7-03: MD parser `<ruby>base<rt>annotation</rt></ruby>` HTML parsing → IR inline ruby field
  - S7-04: HWP CTRL_RUBY already fully implemented — verified 35 existing ruby tests
  - 리뷰 수정: multi-region drain (header+footer both drained), nested `<ruby>` guard
  - 1091 테스트 (949 unit + 23 CLI + 46 integration + 30 roundtrip + 기타, 0 failures)

- [x] v0.5.0 Sprint 8 — test split + serde fix + publish prep (1ffd314):
  - S8-01: `parser_tests.rs` 1622행 → 3파일 분할 (521+390+730행, 69 테스트 보존)
  - S8-02: `HeaderFooterType` `From<String>` normalizer — serde asymmetry 해결
  - S8-03: PROGRESS.md Phase 9b/9c 완료 확인 + 로드맵 정리
  - S8-04: `cargo publish --dry-run` 통과, Cargo.toml v0.5.0, CHANGELOG.md 작성
  - 1094 테스트 (952 unit + 23 CLI + 46 integration + 30 roundtrip + 기타, 0 failures)

### 진행 중

없음

### 미착수
- [ ] `cargo publish` — crates.io 배포

### 완료 (이전 미착수)
- [x] Phase 10: HWPX 라이터 고도화 — Sprint 1 (StyleTemplate), Sprint 4 (header/footer), Sprint 15 (README 반영)

## 중기/장기 개선 로드맵 — 완료 정리

모든 초기 이슈 (CRITICAL~LOW) 및 중기 로드맵 해결 완료. 아래는 잔여 미착수 항목만 기록.

### 미착수 (향후 버전)
- [ ] ParseContext 타입 상태 패턴 또는 빌더 분리 (아키텍처 개선)
- [ ] Reader/Writer trait 정의 (HWP/HWPX/MD 공통 인터페이스)
- [ ] 샘플 HWP/HWPX 파일 기반 통합 테스트 (실제 한글 문서 검증)

### 완료 항목 요약
- 커버리지 80%+ 달성 (Sprint 5: tarpaulin CI + Codecov)
- crates.io 배포 준비 완료 (cargo publish --dry-run 통과)
- 배치 변환 CLI (Sprint 9: batch 서브커맨드)
- 모든 CRITICAL/HIGH/MEDIUM 이슈 해결
- HWP 파서 고도화 (Phase 2~2c: 테이블/이미지/각주/하이퍼링크/DRM/EQEDIT)
- HWPX 파서 고도화 (Phase 3: charPr/li/footnote/BinData/dispatch)
- 아키텍처 분할 (reader.rs 4분할, hwpx/reader.rs 분할, ParseContext 5 sub-structs)

## 변경 이력

### 2026-05-07 — v0.5.0 Sprint 15: README Doc Refresh + Style Template CLI Test

**S15-01: README stale documentation cleanup**:
- style template "interface defined; implementation in progress" → 구현 완료 문구로 변경
- CLI 예제 "full implementation pending" 경고 제거
- format support matrix: Headers/footers → HWPX read/write "yes" 반영
- Known Limitations: "--style not yet applied" 항목 제거
- Headers/footers 제한 사항 → HWP 5.0만 skipped, HWPX 지원 명시

**S15-02: CLI --style end-to-end test**:
- `to_hwpx_style_template_applies_page_dimensions` — YAML style template로 커스텀 page dimensions (70000×90000) 적용 → HWPX ZIP 내 section0.xml 검증

**S15-03: CHANGELOG Sprint 15 entries + date update**:
- v0.5.0 릴리스 날짜 2026-05-04 → 2026-05-07
- Sprint 15 Added/Changed 항목 추가

**S15-04: cargo publish --dry-run**:
- 통과 확인

**리뷰 결과** (0 CRITICAL, 0 HIGH, 1 MEDIUM, 2 LOW):
- M1: CHANGELOG Changed 섹션 Sprint 순서 (cosmetic)
- L1: CLI 테스트가 page dimensions만 검증 (margins/fonts 미포함)
- L2: 문자열 기반 XML assertion (brittle)

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1217 테스트 (0 failures), publish dry-run 통과

### 2026-05-08 — v0.5.0 Sprint 25: map_or + clone_from + Char Patterns

**S25-01: single-char string patterns** (7 instances):
- `eqedit_tests.rs`, `writer_tests_section.rs`, `writer_tests_hyperlink.rs`
- `.contains("x")` → `.contains('x')` — char 매칭이 더 효율적

**S25-02: map().unwrap_or() → map_or()** (11 instances):
- 8개 파일에서 `map(f).unwrap_or(v)` → `map_or(v, f)` 변환
- `hwpx_roundtrip.rs`: `&[]` 타입 추론 실패 → `[].as_slice()` 사용

**S25-03: clone() → clone_from()** (4 instances):
- `hwp/convert.rs`: metadata 필드 복사 시 기존 할당 재사용 가능

**리뷰 결과** (0 CRITICAL, 0 HIGH, 0 MEDIUM, 0 SUGGESTION):
- `map_or`의 인자 순서 `(default, f)` 주의 (학습 포인트)

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures)

### 2026-05-08 — v0.5.0 Sprint 24: Or-Patterns + Control Flow + Misc Pedantic

**S24-01: unnested or-patterns + inline format vars** (hwp/):
- `src/hwp/convert.rs`: 5 or-patterns unnested (`Some('.') | Some(')')` → `Some('.' | ')')`)
- `src/hwp/reader.rs` + `record.rs`: 4 format vars inlined (`format!("{}", var)` → `format!("{var}")`)

**S24-02: redundant continue + else** (md/parser.rs):
- 8 redundant `continue;` 제거 (if/else 체인 끝에서 불필요)
- 1 redundant `else` 블록 제거 (guard clause 패턴)

**S24-03: semicolons + redundant closures**:
- `flush.rs` (3) + `benches/conversion.rs` (5): trailing `;` 추가
- 4 redundant closures → method references (`|x| func(x)` → `func`)
- 벤치마크 roundtrip closure에서 `write_markdown` 반환값 보존 (수동 수정)

**리뷰 결과** (0 CRITICAL, 0 HIGH, 0 MEDIUM, 0 SUGGESTION):
- 벤치마크 `;` 추가 시 `#[must_use]` 반환값 보존 주의 (학습 포인트)

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures)

### 2026-05-08 — v0.5.0 Sprint 23: writeln! Migration + Raw String Cleanup

**S23-01: write!/writeln! migration in md/writer.rs** (20 instances):
- `write_block`, `render_inlines`, `write_table`, `write_html_table`, `write_list` 전체 마이그레이션
- Sprint 22에서 시작한 `write_frontmatter` 마이그레이션 완성 — writer.rs에 `push_str(&format!())` 0건
- `writeln!` for `\n`/`\n\n` endings, `write!` for inline continuations

**S23-02: write!/writeln! migration in hwpx/writer_content.rs** (5 instances):
- `use std::fmt::Write as _;` 추가
- `generate_content_hpf` 함수 내 4건 + `reader_tests_charpr.rs` 1건

**S23-03: unnecessary raw string hashes** (31 instances):
- 10개 테스트 파일에서 `r#"..."#` → `r"..."` (내부에 `"` 없는 경우만)
- XML 속성에 `"` 포함된 문자열은 올바르게 `r#"..."#` 유지

**리뷰 결과** (0 CRITICAL, 0 HIGH, 0 MEDIUM, 1 SUGGESTION):
- S1: 22개 추가 simplifiable raw string 잔존 (다른 파일) — 향후 스프린트 후보

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures)

### 2026-05-08 — v0.5.0 Sprint 22: MIME Refactor + writeln! + Eq Derives

**S22-01: guess_mime_from_name refactor** (pedantic clippy):
- `to_lowercase()` + `ends_with()` → `Path::extension()` + `to_ascii_lowercase()` + match
- 더 관용적이고, 불필요한 전체 문자열 소문자 변환 제거

**S22-02: format! push → writeln!** (md/writer.rs):
- `push_str(&format!(...))` 6건 → `writeln!` 매크로
- `use std::fmt::Write as _;` 추가
- 필드당 임시 String 할당 제거

**S22-03: Eq derive** (IR types):
- Document, Metadata, Section, Block, Inline, TableRow, TableCell, ListItem, Asset, HeaderFooterType
- Sprint 21의 PartialEq에 Eq 추가 — 모든 필드 타입이 Eq 호환

**리뷰 결과** (0 CRITICAL, 0 HIGH, 0 MEDIUM, 1 SUGGESTION):
- S1: dotfile 동작 차이 (.png → octet-stream) — HWPX ZIP 경로에서 발생하지 않으므로 OK

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures), publish dry-run 경고 0건 통과

### 2026-05-08 — v0.5.0 Sprint 21: API Surface Cleanup + PartialEq + #[must_use]

**S21-01: Remove `pub use model::*` from hwp/mod.rs**:
- HWP model types (HwpDocument, FileHeader, CharShape, etc.) no longer exported publicly
- Only `read_hwp()` remains in public API — model types are internal implementation details
- `#![allow(dead_code)]` added to model.rs (HWP spec fields for future use)

**S21-02: Add `PartialEq` to 9 IR types**:
- Document, Metadata, Section, Block, Inline, TableRow, TableCell, ListItem, Asset
- Enables `assert_eq!` in downstream tests; recursive types (Block, ListItem) handled correctly
- `document_default_equals_new` test simplified to use direct `assert_eq!`

**S21-03: Add `#[must_use]` to pure functions and builders**:
- ir.rs: Document::new, Inline::plain/bold/footnote_ref/with_formatting/with_link/with_ruby/with_font_name, ListItem::new, PageLayout::a4_portrait, HeaderFooterType::as_str
- md/parser.rs: parse_markdown
- md/writer.rs: write_markdown
- Not added to Result-returning functions (Result already has #[must_use])

**리뷰 결과** (0 CRITICAL, 0 HIGH, 0 MEDIUM, 4 SUGGESTION):
- S1: `#![allow(dead_code)]` blanket — acceptable for HWP spec module
- S2: 4 missing `#[must_use]` on builders → 수정 완료
- S3: Asset PartialEq byte comparison perf note → 현재 OK, 필요시 custom impl
- S4: document_default_equals_new 테스트 단순화 → 수정 완료

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures), publish dry-run 경고 0건 통과

### 2026-05-08 — v0.5.0 Sprint 20: Test File Splits + CHANGELOG Unreleased

**S20-01: writer_tests_image.rs split** (791→568 + 262행):
- `writer_tests_image_util.rs` 신규 (262행) — base64, MIME, dedup/collision 테스트 13건 추출
- `base64_encode_test` 헬퍼 의도적 복제 (dead_code 방지)

**S20-02: writer_tests_section.rs split** (786→426 + 364행):
- `writer_tests_section_adv.rs` 신규 (364행) — 15건 고급/ID 테스트 추출
- `use super::*;` 패턴으로 부모 헬퍼 접근

**S20-03: CHANGELOG v0.5.0 date → "Unreleased"**:
- Keep a Changelog 규약에 따라 미배포 상태 명시

**리뷰 결과** (0 CRITICAL, 0 HIGH, 1 MEDIUM, 0 LOW):
- M1: 헬퍼 복제 — 의도적, dead_code 경고 방지 목적. 향후 shared test utility 모듈 검토 가능.

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures), publish dry-run 경고 0건 통과

### 2026-05-08 — v0.5.0 Sprint 19: context.rs Split + Example Doc-Test Fences

**S19-01: examples/convert.rs doc-test fences** (Sprint 18 review suggestion):
- 주석 처리된 ConvertOptions 빌더 코드를 ` ```rust,no_run ` 펜스로 래핑
- docs.rs에서 구문 강조 코드 블록으로 렌더링

**S19-02: context.rs split into submodules**:
- `context.rs` (723행) → `context/` 디렉토리 (3파일, 739행)
- `state.rs` (201행): FormattingState, TableState, ListState, FootnoteState, HeaderFooterState, PageLayoutState
- `flush.rs` (350행): apply_charpr_attrs, flush 함수들, StagedBlock, group_list_paragraphs
- `mod.rs` (188행): ParseContext struct, RubyPart, dispatch 메서드, re-exports
- `reader.rs`: `#[path = "context/mod.rs"]` 경로 업데이트

**S19-03: cargo publish --dry-run**:
- 경고 0건 통과

**리뷰 결과** (0 CRITICAL, 0 HIGH, 0 MEDIUM, 1 LOW):
- L1: `#[path]` 불필요 제안 — 실제로는 file-based module이므로 필요

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures), publish dry-run 경고 0건 통과

### 2026-05-07 — v0.5.0 Sprint 18: docs.rs Example + Roadmap Cleanup

**S18-01: examples/convert.rs** (docs.rs library showcase):
- ConvertOptions builder API (commented-out demo, no file I/O)
- IR Document 프로그래매틱 구성 (Heading + bold Paragraph + Table)
- `write_markdown(&doc, false)` 렌더링 + 출력
- zero unwrap, zero file I/O, `cargo run --example convert` 독립 실행

**S18-02: PROGRESS.md roadmap cleanup**:
- 중기/장기 섹션 ~65행 → ~15행 압축 (완료 항목 제거, 잔여 3건만 유지)

**S18-03: cargo publish --dry-run**:
- 104 files, 240.5 KiB, 경고 0건 통과

**리뷰 결과** (0 CRITICAL, 0 HIGH, 0 MEDIUM, 1 LOW):
- L1: mut + push 패턴 — Rust 소유권 모델에서 안전하므로 조치 불필요

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures), publish dry-run 경고 0건 통과

### 2026-05-07 — v0.5.0 Sprint 17: CLI Test Split + lib.rs URL Fix

**S17-01: CLI style template test helper + split** (Sprint 16 M1):
- `tests/cli_style.rs` 신규 (155행) — 3 style template 테스트 추출
- `run_to_hwpx_with_style(md, yaml)` 공유 헬퍼 (tempdir + CLI 실행 + ZIP 경로 반환)
- `tests/cli.rs` 924→751행 (800행 가이드라인 준수)

**S17-02: lib.rs README URL 수정**:
- `CasterLink/hwp2md` → `hephaex/hwp2md` (docs.rs 404 방지)

**S17-03: cargo publish --dry-run**:
- 경고 0건 통과

**리뷰 결과** (0 CRITICAL, 0 HIGH, 0 MEDIUM, 0 LOW):
- 클린 승인. ZIP 읽기 패턴 추가 헬퍼 추출 선택적 제안만.

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures), publish dry-run 경고 0건 통과

### 2026-05-07 — v0.5.0 Sprint 16: Style Template CLI Tests + Publish Fix

**S16-01: Style template CLI tests — margins + font** (Sprint 15 L1):
- `to_hwpx_style_template_applies_margins` — margin 값 (8000/8000/6000/6000) section XML 검증
- `to_hwpx_style_template_applies_custom_font` — "맑은 고딕" font name header.xml 검증

**S16-02: Fix cargo publish benchmark warning**:
- Cargo.toml exclude에서 `"benches/"` 제거 — `[[bench]]` 참조 경고 해결

**S16-03: PROGRESS.md Phase 10 status cleanup**:
- Phase 10 "HWPX 라이터 고도화" 미착수 → 완료 처리 (Sprint 1/4/15에서 구현 완료)

**S16-04: cargo publish --dry-run**:
- 경고 0건 통과

**리뷰 결과** (0 CRITICAL, 0 HIGH, 1 MEDIUM, 2 LOW):
- M1: 3개 style template 테스트 간 setup 중복 → 헬퍼 추출 권장
- L1: Sprint 15 → 16 comment 오타 → 수정 완료
- L2: 문자열 기반 XML assertion (brittle)

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures), publish dry-run 경고 0건 통과

### 2026-05-07 — v0.5.0 Sprint 14: Image Asset Tests + README CLI Docs

**S14-01: HwpxFixture.bin_data()** (테스트 인프라):
- `bin_data(name, data)` 빌더 메서드 — BinData/ ZIP 엔트리 임베딩
- 2 integration 테스트: read_hwpx BinData assets 검증 + write_assets 디스크 추출 검증

**S14-02: CLI convert --assets-dir 이미지 추출 테스트** (Sprint 13 M1):
- HwpxFixture로 이미지 포함 HWPX 생성 → CLI convert --assets-dir → 파일 추출 + 바이트 일치 검증

**S14-03: README CLI 문서 업데이트**:
- `convert` 서브커맨드: --assets-dir, --frontmatter, --style, --force 문서화
- `batch` 서브커맨드: --output-dir, --assets-dir, --frontmatter, --force 문서화

**S14-04: cargo publish --dry-run**:
- 통과 확인

**리뷰 결과** (0 CRITICAL, 0 HIGH, 1 MEDIUM, 3 LOW):
- M1: README batch 예제 --output-dir 장형 누락 → 수정 완료
- L1: 불필요한 _doc 바인딩 → 제거

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1216 테스트 (0 failures), publish dry-run 통과

### 2026-05-07 — v0.5.0 Sprint 13: CLI Completeness + Unwrap Consistency

**S13-01: common/mod.rs unwrap→expect** (Sprint 12 L1):
- 3 bare `.unwrap()` → `.expect("descriptive message")` for test diagnostics

**S13-02: convert CLI option parity**:
- `--assets-dir` 플래그 추가 (HWP/HWPX→MD 이미지 추출 디렉토리)
- `--frontmatter` 플래그 추가 (YAML 메타데이터 포함)
- `--style` 플래그 추가 (MD→HWPX 스타일 템플릿)
- ConvertOptions builder 직접 사용으로 convert_auto 대체

**S13-03: batch CLI --assets-dir**:
- `--assets-dir` 플래그 추가 — per-file `<base>/<stem>/` 서브디렉토리 자동 생성
- run_batch 시그니처에 assets_dir 파라미터 추가

**S13-04: cargo publish --dry-run**:
- 101 files, 236.4 KiB compressed, 통과

**리뷰 결과** (0 CRITICAL, 0 HIGH, 2 MEDIUM):
- M1: convert --assets-dir 테스트가 smoke test 수준 (이미지 포함 fixture 없음)
- M2: run_batch 호출 라인 길이 → 수정 완료

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1213 테스트 (0 failures), publish dry-run 통과

### 2026-05-06 — v0.5.0 Sprint 12: Common Helpers + List Split + Publish Prep

**S12-01: tests/common/mod.rs 공통 헬퍼 추출** (Sprint 11 M1):
- 7개 통합 테스트 파일에서 중복된 헬퍼 함수를 `tests/common/mod.rs`로 추출
- 9개 헬퍼: `cargo_bin`, `make_hwpx`, `plain`, `make_doc`, `first_blocks`, `collect_all_text`, `md_to_hwpx_to_ir`, `md_to_hwpx_to_md`, `ir_to_hwpx_to_md`
- `#[path = "common/mod.rs"] mod common;` 패턴으로 참조
- Clippy `empty_line_after_doc_comments` 수정: `///` → `//` (모듈 레벨 주석)

**S12-02: writer_tests_list.rs 분할** (Sprint 11 L1):
- `writer_tests_list.rs` (871행) → `writer_tests_list.rs` (622행) + `writer_tests_list_adv.rs` (251행)
- 기본 리스트 테스트 유지, 고급/roundtrip/task-list 테스트 7건 추출

**S12-03: cargo publish --dry-run**:
- 통과 확인, v0.5.0 배포 준비 완료

**리뷰 결과** (0 CRITICAL, 0 HIGH, 1 MEDIUM, 1 LOW):
- M1: `make_doc()` mutation (acceptable — test helper)
- L1: common/mod.rs unwrap/expect 일관성

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1209 테스트 (0 failures), publish dry-run 통과

### 2026-05-04 — v0.5.0 Sprint 11: Trace Logging + Test Splits + Orphan Cleanup

**S11-01: Batch trace logging** (Sprint 10 M2):
- `run_batch()`: hidden file, symlink skip 시 `tracing::debug!` 로깅 추가

**S11-02: tests/cli.rs 분할** (Sprint 10 M3):
- `tests/cli.rs` (975행) → `cli.rs` (635행, 23 tests) + `cli_batch.rs` (385행, 10 tests)
- 공통 헬퍼 (`cargo_bin`, `make_hwpx`) 양쪽 복사

**S11-03: tests/roundtrip.rs 분할** (Sprint 10 M3):
- `tests/roundtrip.rs` (1102행) → `roundtrip.rs` (622행, 20 tests) + `roundtrip_stability.rs` (505행, 10 tests)
- 공통 헬퍼 (`plain`, `make_doc`, `first_blocks`) 양쪽 복사

**S11-04: convert_tests 분할 + orphan 삭제**:
- `src/hwp/convert_tests.rs` (950행) orphan 삭제 — 이미 5개 분할 파일에 전수 포함
- `src/convert_tests.rs` (808행) → `convert_tests.rs` (517행, 33 tests) + `convert_tests_count.rs` (293행, 21 tests)

**S11-05: cargo publish --dry-run**:
- 99 files, 235.2 KiB compressed, 통과

**리뷰 결과** (0 CRITICAL, 0 HIGH, 1 MEDIUM, 1 LOW):
- M1: Integration test 헬퍼 중복 — tests/common/mod.rs 추출 권장
- L1: writer_tests_list.rs (871행) 800행 초과 (기존)

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1209 테스트 (0 failures), publish dry-run 통과

### 2026-05-04 — v0.5.0 Sprint 10: Batch Hardening + File Splits

**S10-01: Batch hidden file/symlink guard** (Sprint 9 M1):
- `run_batch()` dir-walk: dotfile (`starts_with('.')`) 필터 + symlink (`file_type()?.is_symlink()`) 필터
- 2 CLI 테스트: `batch_skips_hidden_files`, `batch_skips_symlinks` (unix-only)

**S10-02: Batch separate skip/error counters** (Sprint 9 L1):
- `errors` → `skipped` + `failed` 분리
- 출력 형식: "Batch complete: N converted, M skipped, F failed"
- skip-existing 은 skipped, 변환 실패는 failed로 분류

**S10-03: hwpx/writer.rs 분할** (코드 품질):
- `writer.rs` (822행) → `writer.rs` (390행) + `writer_content.rs` (453행)
- 이미지 수집, base64, 정적 XML 생성 함수 추출
- `#[path]` + `pub(super)` 패턴, `#[cfg(test)]` re-export

**S10-04: Orphan reader_tests.rs 정리** (코드 품질):
- 이전 분할에서 orphan된 `reader_tests.rs` (1296행) 삭제
- 누락된 4 page-break 테스트 `reader_tests_structure.rs`에 복구
- Clippy `manual_contains` 수정

**S10-05: hwpx_roundtrip.rs 분할** (코드 품질):
- `hwpx_roundtrip.rs` (1288행) → 3파일 (275+536+630행)
- 46 roundtrip 테스트 전수 보존

**리뷰 결과** (0 CRITICAL, 0 HIGH, 3 MEDIUM):
- M1: `file_type()?.is_symlink()` Windows 호환 제한
- M2: Hidden/symlink skip 시 trace 로깅 부재
- M3: `tests/cli.rs` (975행), `tests/roundtrip.rs` (1102행) 800행 초과

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1209 테스트 (0 failures)

### 2026-05-04 — v0.5.0 Sprint 9: Batch CLI + Test Splits + From Edge Cases

**S9-01: From<&str> + edge case tests** (Sprint 8 M1):
- `From<String>` → `From<&str>` 위임 패턴 적용
- 4 edge case 테스트: empty, whitespace, capitalized, str ref known values

**S9-02: convert_tests.rs 추출** (Sprint 8 L1):
- `convert.rs` 인라인 테스트 → `convert_tests.rs` 분리 (1316→508+808행)
- 54 테스트 전수 보존, `#[path]` 모듈 패턴

**S9-03: batch CLI** (신규):
- `hwp2md batch <input_dir> [output_dir]` 서브커맨드
- `--force` 덮어쓰기, `--frontmatter` 메타데이터 옵션
- 입력 검증: 미존재/비디렉토리 경로 에러 처리
- 8 CLI 통합 테스트

**S9-04: writer_tests 분할** (코드 품질):
- `writer_tests.rs` (1395행) → 2파일 (718+705행)
- 97 테스트 전수 보존, `#[path]` 모듈 패턴

**리뷰 결과** (0 CRITICAL, 0 HIGH, 2 MEDIUM, 1 LOW):
- M1: Hidden files/symlinks not filtered in batch
- M2: Output path not canonicalized
- L1: Skipped-existing counted as errors

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1203 테스트 (0 failures)

### 2026-05-04 — v0.5.0 Sprint 8: Test Split + Serde Fix + Publish Prep

**S8-01: parser_tests.rs 분할** (Sprint 7 L1):
- `parser_tests.rs` (1622행) → 3 파일: `parser_tests.rs` (521행, 31 core), `parser_tests_inline.rs` (390행, 19 inline), `parser_tests_marker.rs` (730행, 19 marker)
- `#[cfg(test)] #[path]` 패턴, `pub(super)` 헬퍼 공유

**S8-02: HeaderFooterType serde 수정** (Sprint 7 M2):
- `From<String>` impl: `"both"/"even"/"odd"` → enum variant 자동 정규화
- `handlers.rs` simplified: explicit match → `.into()`
- 3 새 테스트

**S8-03: PROGRESS.md 정리**:
- Phase 9b (crypto) + 9c (lenient CFB + ruby) 이미 구현 확인 → 완료 처리

**S8-04: v0.5.0 배포 준비**:
- Cargo.toml v0.5.0, README dependency `"0.5"`, CHANGELOG.md 작성
- `cargo publish --dry-run` 통과 (93 files, 240.8 KiB)

**리뷰 결과** (0 CRITICAL, 0 HIGH, 1 MEDIUM, 1 LOW):
- M1: `From<String>` edge case 미테스트 (empty, whitespace, capitalized)
- L1: `From<&str>` 편의 impl 부재

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1094 테스트 (0 failures), publish dry-run 통과

### 2026-05-04 — v0.5.0 Sprint 7: Unclosed Marker Fallback + HeaderFooterType Enum + Ruby HTML Parsing

**S7-01: Unclosed marker fallback** (Sprint 6 리뷰 M1):
- `parse_markdown()` Region state machine: EOF에서 `region != Body`이면 header_blocks + footer_blocks 모두 body로 drain
- `tracing::warn!` 경고 출력
- 4 테스트: unclosed header/footer, empty marker region, interleaved markers

**S7-02: HeaderFooterType enum** (Sprint 6 리뷰 M3):
- `HeaderFooterType` enum: `Both`, `Even`, `Odd`, `Other(String)` — `#[serde(rename_all = "camelCase")]`
- `as_str()` 메서드 + serde roundtrip 테스트
- HWPX reader/writer/tests 전수 마이그레이션

**S7-03: Ruby HTML parsing** (Phase 9c):
- MD parser: `<ruby>`, `<rt>`, `</rt>`, `</ruby>` 태그 상태 머신
- `ruby_base_start` 인덱스로 복수 base inline에 annotation 일괄 적용
- 4 테스트: basic ruby, roundtrip, without rt, bold base

**S7-04: HWP CTRL_RUBY** — 이미 완전 구현 확인 (35 ruby 테스트 통과)

**리뷰 수정** (1 HIGH + 2 MEDIUM):
- HIGH: nested `<ruby>` guard 추가 (tracing::warn + skip)
- MEDIUM: multi-region drain — unclosed marker 시 header+footer 모두 body로 drain (단일 region만 drain하던 버그 수정)

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1091 테스트 (0 failures)

### 2026-05-03 — v0.5.0 Sprint 6: MD Header/Footer Round-trip + DRM Integration Test + HeaderFooter Type

**S6-01: MD parser header/footer round-trip** (Sprint 4 리뷰 M2):
- `parse_markdown()` Region state machine: `enum Region { Body, Header, Footer }`
- comrak AST level에서 `<!-- header -->` / `<!-- /header -->` / `<!-- footer -->` / `<!-- /footer -->` 마커 감지
- 마커 안의 블록을 header/footer/body 버킷으로 라우팅, Section에 할당
- `html_comment_keyword()` 공유 헬퍼: pagebreak 감지와 통합
- 6 unit + 1 roundtrip 테스트

**S6-02: DrmProtected integration test** (Sprint 5 리뷰 M2):
- `cfb::CompoundFile::create()` + `create_stream("/FileHeader")` 로 valid CFB fixture 생성
- FileHeader: HWP signature + version 5.1.0.0 + properties `0x10` (has_drm bit)
- `read_hwp()` → `Hwp2MdError::DrmProtected` 반환 검증 + 에러 메시지 검증
- 2 integration 테스트

**S6-03: headerFooter type attribute** (Sprint 4 리뷰 L1):
- `ir::Section`에 `header_footer_type: Option<String>` 필드 추가
- HWPX reader: `<hp:headerFooter type="both|even|odd">` 속성 파싱 → `hf_type` 상태 저장 → Section 전달
- HWPX writer: `header_footer_type` 존재 시 `<hp:headerFooter type="...">` 출력
- 4 reader + 4 writer round-trip 테스트

**리뷰 결과** (0 CRITICAL, 0 HIGH, 3 MEDIUM, 3 LOW):
- M1: Unclosed marker silently misroutes content — 향후 fallback 추가
- M2: Interleaved markers produce surprising bucket assignment — 테스트/명세 추가
- M3: `header_footer_type` is `Option<String>` — enum 타입화 고려

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1082 테스트 (0 failures)

### 2026-05-03 — v0.5.0 Sprint 5: Structured Errors + Cross-platform CI + Coverage

**Phase C-4: Structured error payloads**:
- `Hwp2MdError::OutputExists { path: PathBuf }` — `--force` 가드에서 `UnsupportedFormat` 남용 교체
- `Hwp2MdError::DrmProtected { path: PathBuf }` — 암호화 HWP에서 `HwpParse(String)` 남용 교체
- `convert_auto`, `ConvertOptions::execute` → `OutputExists` 사용
- `hwp::reader::parse_hwp_file` → `DrmProtected` 사용
- Display 포맷 검증 테스트 + 패턴 매칭 검증 테스트

**Phase D-2: Cross-platform CI**:
- GitHub Actions 매트릭스: ubuntu-latest + windows-latest + macos-latest
- MSRV 1.75.0 핀, `fail-fast: false`
- lint (clippy + fmt) 분리 → ubuntu only

**Phase D-3: Coverage reporting**:
- cargo-tarpaulin (ubuntu, stable toolchain) + cobertura.xml 출력
- Codecov 업로드 (codecov-action@v4, `fail_ci_if_error: false`)
- README.md Codecov 배지 추가

**리뷰 수정** (1 CRITICAL + 1 HIGH):
- CRITICAL: `read_hwp()` lenient fallback이 `DrmProtected` 에러를 삼킴 → 조기 반환 가드 추가
- HIGH: `cargo install cargo-tarpaulin` → `taiki-e/install-action@v2` 프리빌트 바이너리 (CI 5-10분 절약)

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1065 테스트 (0 failures)

### 2026-05-02 — v0.5.0 Sprint 4: Header/Footer + --force + ConvertOptions

**Phase B-4: Header/footer reading**:
- `ir::Section`에 `header: Option<Vec<Block>>`, `footer: Option<Vec<Block>>` 추가 (`#[serde(default, skip_serializing_if)]`)
- HWPX reader: `<hp:headerFooter>` → `<hp:header>`/`<hp:footer>` 파싱, `HeaderFooterState` sub-struct
- HWPX writer: `<hp:headerFooter>` 요소 `<hp:secPr>` 앞에 출력 (header/footer가 존재할 때만)
- MD writer: `<!-- header -->` / `<!-- /header -->`, `<!-- footer -->` / `<!-- /footer -->` HTML 코멘트 마커
- `push_block_scoped`, `flush_active_paragraph_scope`에 header/footer scope 라우팅 추가
- 8 reader + 5 writer + 3 md + 2 roundtrip 테스트

**Sprint 3 리뷰 M3: --force 플래그**:
- `convert` 서브커맨드에 `--force` 플래그 추가
- `convert_auto(input, output, force: bool)` — 기존 출력 파일 존재 시 `force=false`면 에러 반환
- 3 단위 + 2 CLI 통합 테스트

**Phase C-3: ConvertOptions builder**:
- `ConvertOptions<'a>` fluent builder: `new()` → `.assets_dir()` → `.frontmatter()` → `.style()` → `.force()` → `.execute()`
- `hwp2md::ConvertOptions` 크레이트 루트 re-export + `lib.rs` 독스트링 업데이트
- 14 builder 단위 테스트

**리뷰 수정** (3 HIGH):
- H1: `push_block_scoped` — header/footer 분기를 footnote 앞에 추가 (이미지 누출 방지)
- H2: `flush_active_paragraph_scope` — header/footer 분기 추가 (page break 누출 방지)
- H3: `headerFooter` 시작 시 `header_blocks`/`footer_blocks` 초기화 (stale 블록 방지)
- 회귀 테스트: `image_in_header_stays_in_header`, `page_break_in_footer_stays_in_footer`

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1062 테스트 (0 failures)

### 2026-05-02 — v0.5.0 Sprint 3: Page Break + Auto-detect + FileTooLarge

**Phase B-3: Page break round-trip**:
- `ir::Block::PageBreak` 신규 변형 추가
- HWPX 리더: `<hp:ctrl id="newPage|pageBreak|cnpb"/>` → `Block::PageBreak` (footnote/list/cell scope 모두 지원)
- HWPX 라이터: `<hp:p><hp:run><hp:ctrl id="newPage"/></hp:run></hp:p>` 출력 (hp:t 텍스트 노드 없음)
- MD 라이터: `<!-- pagebreak -->` HTML 코멘트 마커 (렌더링 시 비가시)
- MD 파서: `<!-- pagebreak -->` HTML 블록 → `Block::PageBreak` (대소문자 무관)
- Block enum 모든 exhaustive match 부지에 PageBreak 추가 (count_chars/resolve_block_bin_refs/collect_*/cell_to_text)
- 12 신규 테스트: writer 2 + parser 3 + hwpx writer 1 + hwpx reader 3 + roundtrip 1 + IR/dispatch

**Phase C-2: Format auto-detection**:
- `convert <input> <output>` 신규 CLI 서브커맨드
- `convert::convert_auto()` + `FormatKind { Hwp, Hwpx, Markdown, Unknown }` 분류기
- 지원 매핑: `.hwp/.hwpx → .md/.markdown`, `.md/.markdown → .hwpx`
- 동일 포맷 / 알 수 없는 확장자 → "cannot infer conversion direction" 에러 메시지
- 8 신규 테스트: 단위 7 (분류기 + 정상 + 거부 + 대소문자) + CLI 4

**Sprint 2 리뷰 수정 (M1)**:
- 전용 `Hwp2MdError::FileTooLarge { path: String, size: u64, limit: u64 }` 추가
- `check()`의 256MB 가드가 `UnsupportedFormat` 대신 `FileTooLarge` 반환
- 기존 oversize 테스트 → variant 패턴 매칭으로 강화 + Display 포맷 검증 1건 추가

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1021 테스트 (0 failures)

### 2026-04-27 — v0.5.0 Sprint 2: Benchmarks + Check Guard + Tests (c616de1)

**Phase D-1: Criterion 벤치마크**:
- benches/conversion.rs: 5 벤치마크 (MD→IR, IR→MD, IR→HWPX, HWPX→IR, 풀 라운드트립)
- 대표 입력: 한국어 제목+단락 3개+테이블 3×3+코드블록+목록 5개+서식
- 결과: MD→IR 107µs, IR→MD 30µs, IR→HWPX 527µs, HWPX→IR 139µs, roundtrip 781µs
- criterion 0.5 dev-dependency 추가, HTML 리포트 활성화

**Sprint 1 리뷰 MEDIUM 수정**:
- M3 수정: check() MD 파일 크기 가드 (MAX_MD_FILE_SIZE = 256MB)
- fs::metadata().len() 사전 검사, 초과 시 UnsupportedFormat 에러
- 3 새 테스트: 상수 값 검증, 정상 파일 통과, 초과 거부 경계값

**Sprint 1 리뷰 LOW 수정**:
- ordered task list writer 테스트 추가 (ordered_task_list_items)
- ordered: true + mixed checked/unchecked/normal 항목 검증

**검증**: cargo check 0 에러, clippy 0 경고, 1001 테스트 (0 failures)

### 2026-04-27 — v0.5.0 Sprint 1: 리팩토링 + 스타일 + Task List + Check (139ab22)

**Phase A-1: ParseContext 리팩토링**:
- 37 flat fields → 5 sub-structs (FormattingState, TableState, ListState, FootnoteState, PageLayoutState)
- flush_inlines_to_blocks: 11 params → 4 (buffers + &FormattingState)
- make_inline() 헬퍼 추출

**Phase A-2: PageLayout 파싱 중복 제거**:
- pageSize/margin/pagePr 파싱을 PageLayoutState 메서드로 추출
- handle_start_element/handle_empty_element 50행 중복 제거

**Phase B-1: YAML 스타일 템플릿**:
- StyleTemplate (serde_yml 0.0.12): page dimensions/margins, font overrides, heading line_spacing
- RefTables에 code_font + style 통합, CharPrKey 코드 폰트 파라미터화
- validate(): zero width/height/line_spacing 거부
- 6 page layout + 3 validation + 5 style integration 테스트

**Phase B-2: Task List 지원**:
- ListItem.checked: Option<bool> (None=일반, Some(false)=☐, Some(true)=☑)
- comrak tasklist extension 활성화, NodeValue::TaskItem 매핑
- MD writer: `[x]`/`[ ]` 접두사, HWPX writer: ☑/☐ 유니코드
- 6 parser + 5 writer + 4 HWPX 테스트

**Phase C-1: --check 서브커맨드**:
- convert::check(): 확장자 디스패치 → reader 호출 (출력 없이 파싱 검증)
- CLI Commands::Check 변형 추가
- 8 unit + 6 CLI 통합 테스트

**기타 수정**:
- serde_yaml (deprecated) → serde_yml 0.0.12
- PageLayout: Copy derive 추가 (clippy clone-on-copy 해결)

**검증**: cargo check 0 에러, clippy 0 경고, 997 테스트 (0 failures)

### 2026-04-23 — Phase 9a: rhwp 비교 분석 + MUST 버그 수정 (d1cab3d + d108d62)

**rhwp 비교 분석**:
- rhwp (github.com/edwardkim/rhwp): 148K LOC, HWP 읽기/쓰기/렌더링/WASM
- 주요 갭: 배포문서 복호화, Lenient CFB, 50+ 컨트롤, 891+ 테스트
- 즉시 수정 가능한 MUST 버그 3건 발견

**MUST 버그 수정 3건**:
- CharShape: superscript/subscript 파싱 (attr bits 16-17) — shapes.rs
- ParaShape: heading_type (outline level) 파싱 (attr1 bits 24-25) — shapes.rs
- HWPX: charPr supscript 속성 파싱 + 4개 flush 함수 전파 — context.rs

**6 신규 테스트**:
- parse_char_shape_superscript_flag, _subscript_flag, _bold_and_superscript
- parse_para_shape_heading_type_outline, _level_3, _no_heading

**개선 로드맵 (SHOULD 이상)**:
1. 배포문서 복호화 (AES-128, ~500 LOC)
2. Lenient CFB 폴백 (~300 LOC)
3. Ruby 텍스트 컨트롤 (~200 LOC)
4. ParaShape numbering_id 리스트 감지 (~100 LOC)
5. Table cell 메타데이터 (~150 LOC)

**검증**: cargo check 0 에러, clippy 0 경고, 510 테스트 (448 unit + 11 CLI + 24 integration + 27 roundtrip)

### 2026-04-22 — Phase 8: 테스트 분리 + 통합 테스트 인프라 + 토크나이저 버그 수정 (ac63b6f)

**eqedit.rs 테스트 분리**:
- 42 단위 테스트 → src/hwp/eqedit_tests.rs (`#[cfg(test)] #[path]` 패턴)
- eqedit.rs: 829→563행 (800행 가이드라인 준수)
- 2 신규 에지케이스 테스트: deep_nesting (50중첩), unmatched_closing_brace

**HwpxFixture 빌더 (tests/fixtures/mod.rs)**:
- 프로그래매틱 HWPX ZIP 생성: mimetype, container.xml, content.hpf, section0.xml, header.xml
- 헬퍼: para_xml(), heading_xml(), table_2x2_xml(), styled_run_xml()
- write_to_tempfile(): tempfile 기반 임시 파일 생성

**24 통합 테스트 (tests/integration.rs)**:
- 빈 문서, 메타데이터(title/author), 단락, 복수 단락, 제목(1-6), 테이블(2×2)
- 서식(bold+italic), 혼합 컨텐츠, 다수 단락(5개), 매우 긴 단락
- API: hwpx_to_markdown/markdown_to_hwpx 직접 호출
- ZIP 구조: mimetype 존재, section0.xml, content.hpf
- 엣지: 빈 제목/셀, 특수문자(< > & ")

**토크나이저 버그 수정**:
- 무한 루프: tokenise()에서 bare '}' 문자를 처리하지 않아 i가 전진하지 않음
- 수정: `} ` → `Token::Word("}"), i += 1` 핸들러 추가
- 리뷰에서 제안된 `saturating_sub` 수정이 원인이 아님 — 토크나이저 자체 결함

**검증**: cargo check 0 에러, clippy 0 경고, 504 테스트 (442 unit + 11 CLI + 24 integration + 27 roundtrip)

### 2026-04-22 — Phase 5: Parser 고도화 + 테스트 커버리지 (8360c8f)

**MD parser 개선**:
- InlineStyle: underline, subscript 필드 추가
- HtmlInline 상태머신: `<u>`/`<sub>` open/close 태그 추적
- close-tag: 부모 스타일 값으로 복원 (중첩 시 외부 플래그 보존)
- 16 단위 테스트: frontmatter(4), footnote/math/image(4), 인라인 스타일(7), 통합(1)

**CLI 통합 테스트 (11건)**:
- --help, --version, 서브커맨드 help (×3)
- 에러: 존재하지 않는 파일, 미지원 형식
- End-to-end: MD→HWPX→MD 파이프라인, HWPX ZIP 구조 검증

**Roundtrip 테스트 (12건 추가, 총 26건)**:
- IR→MD→IR: display math, footnote, image, nested list, frontmatter
- MD→IR→MD 안정성: math, footnote, escaped text, image, HTML colspan table
- 엣지: empty document, Korean unicode

**리뷰**:
- 리뷰어가 parser.rs를 실수로 revert → 재적용 후 검증
- LOW 3건: nested `<u><sub>` 조합 테스트, unclosed tag 테스트, code block 2-pass

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 359 테스트 (322 unit + 11 CLI + 26 roundtrip)

### 2026-04-22 — Phase 4: Writer 고도화 + MD 안전성 (c9780c5)

**HWPX writer**:
- cellAddr colspan/rowspan: `<hp:cellAddr colSpan="N" rowSpan="N"/>` 출력 (span > 1)
- 1×1 셀은 cellAddr 미출력 (OWPML 스키마 준수)
- 테스트 분리: writer_tests.rs (writer.rs 814→279행)

**MD writer**:
- escape_inline(): 7종 GFM 메타문자 (\\ \` \* \_ \~ \[ \]) 이스케이프
- 빈 텍스트 guard: bold/italic/strikethrough/underline/super/sub 마커 생략
- cell_to_text: `_ => {}` catch-all → 9개 Block variant 명시
- 테스트 분리: writer_tests.rs (writer.rs 876→310행)

**Block enum 전수 감사**:
- 코드베이스 전체 ir::Block match에서 catch-all 0건 확인
- cell_to_text가 유일한 잔여 catch-all이었음 → 수정 완료

**리뷰**:
- MEDIUM 3건: md/writer 분할(완료), #/> 이스케이프(HWP 저빈도, 미착수), cellAddr colAddr/rowAddr(미착수)

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 320 테스트 (306 unit + 14 integration)

### 2026-04-22 — Phase 3b: 모듈 분할 + 테스트 확충 (35ea51f)

**reader.rs 분할**:
- 테스트 968행 → src/hwpx/reader_tests.rs 분리 (`#[path]` 패턴)
- reader.rs: 1895→931행 (800행 가이드라인 근접)
- ParseContext dispatch: active_text_buf(), push_inline(), push_block_scoped()
- 5+ 핸들러에서 if/else 라우팅 체인 제거

**writer.rs 테스트 (31건)**:
- generate_section_xml: paragraph, heading, charPr(bold/italic/underline/strikeout), image, table, math, list, footnote, blockquote, code block, horizontal rule, ordering
- write_hwpx ZIP: required entries, mimetype stored uncompressed, sections, BinData asset, metadata, content.hpf
- colspan/rowspan limitation documented (cellAddr not emitted)

**convert.rs 테스트 (39건) + 버그 수정**:
- count_chars exhaustive match: `_ => 0` 제거, Footnote content 재귀 추가
- .len() → .chars().count() (CodeBlock/Math, CJK 안전)
- write_assets: 정상 추출 + path traversal 방어 검증
- orchestrator: 확장자 검증, md→hwpx→md 라운드트립

**리뷰 수정 (2 HIGH)**:
- H1: 디스패치 우선순위 통일 (footnote > list_item > cell) 문서화
- H2: count_chars .len() → .chars().count() 일관성

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 302 테스트 (288 unit + 14 integration)

### 2026-04-22 — Phase 3: HWPX 고도화 (bf57712)

**charPr 리팩토링**:
- apply_charpr_attrs() 헬퍼로 Start/Empty 핸들러 중복 제거

**목록 `<li>` 핸들러**:
- ParseContext에 in_list_item, list_item_blocks/inlines/text 필드 추가
- flush_list_item_paragraph() 헬퍼, `<li>`/`<hp:li>` Start/End 핸들링
- `<ol>/<ul>` → List 블록에 실제 ListItem 생성

**각주/미주 파싱**:
- ParseContext에 in_footnote, footnote_id/blocks/inlines/text 필드 추가
- `<hp:fn>`, `<hp:en>`, `<hp:footnote>`, `<hp:endnote>` → IR Block::Footnote
- `<hp:noteRef noteId="">`, `<hp:ctrl id="fn" idRef="">` → inline footnote_ref

**BinData 참조 해결**:
- build_bin_map(): BinData/ ZIP 경로에서 stem→full_path HashMap 구축
- resolve_bin_refs(): IR 블록 트리 재귀 순회, binaryItemIDRef 치환
- read_hwpx()에서 섹션 파싱 후 자동 적용

**리뷰 수정 (4 HIGH)**:
- H1: resolve_block_bin_refs에 Footnote/List/BlockQuote 재귀 추가 + exhaustive match
- H2: lineBreak → in_list_item 라우팅 추가
- H3: noteRef/ctrl → in_footnote/in_list_item 라우팅 추가
- H4: img/picture → in_footnote/in_list_item 라우팅 추가
- 교차 테스트 5건 (image/linebreak in footnote/list, resolve in footnote/list)

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 243 테스트 (229 unit + 14 integration)

### 2026-04-22 — Phase 2d: HWPX 테스트 + Dead code 정리 (5e297d7)

**HWPX reader 테스트**:
- 42 단위 테스트 추가 (기존 8건 → 50건)
- parse_section_xml: 단락, 제목(styleIDRef), charPr(bold/italic/underline/strikeout),
  테이블(colspan/rowspan/cellAddr), 이미지, 수식, 목록, lineBreak
- guess_mime_from_name: 8개 확장자 + 대소문자 + 미지 확장자 (13건)
- 리뷰 수정: underline/strikeout 테스트 추가, colSpan=0 기본값 테스트, 테스트명 정정

**Dead code 정리**:
- record.rs: 6개 경고 → per-item `#[allow(dead_code)]` (HWP 스펙 상수, 향후 사용)
- rustfmt 정리: reader.rs, summary.rs 주석 정렬

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 229 테스트 (215 unit + 14 integration)

### 2026-04-22 — Phase 2c: EQEDIT→LaTeX + GitHub Actions CI (37d2645 + da92270)

**EQEDIT→LaTeX 변환기**:
- eqedit.rs 모듈: 토크나이저 + 6단계 멀티패스 변환 파이프라인
- 지원: 분수(over), 그리스 문자(60+), 연산자, 루트(sqrt/root), 매트릭스, 파일, 구분자(left/right)
- MAX_RECURSION_DEPTH=32로 악의적 입력 스택 오버플로우 방지
- convert.rs: Equation control → eqedit_to_latex → IR Math 블록 (단일 변환 지점)
- 42 단위 테스트

**GitHub Actions CI**:
- Rust 1.75.0 MSRV, dtolnay/rust-toolchain, Swatinem/rust-cache
- cargo check + clippy (-D warnings) + fmt check + test

**리뷰 수정**:
- C1: 무한 재귀 방지 (depth counter)
- H1: 이중 변환 제거 (control.rs→convert.rs 단일 지점)
- H2: 토크나이저 right-delimiter 오탐 제거

**검증**: cargo check 0 에러, clippy 0 경고, 187 테스트 (173 unit + 14 integration)

### 2026-04-22 — Phase 2b: 서브모듈 분할 + 하이퍼링크 + DRM (71e54ea)

**아키텍처 리팩토링**:
- reader.rs (2057L) → reader.rs (828L) + control.rs (781L) + convert.rs (434L) + summary.rs (238L)
- 모든 파일 800L 가이드라인 준수 (reader.rs 828L는 테스트 포함)

**Phase 2b 기능**:
- 하이퍼링크 URL 추출: CTRL_HYPERLINK → parse_hyperlink_url → IR Paragraph + linked Inline
- parse_summary_bytes 추출: OLE2 파싱 로직 분리, 바이트 버퍼 기반 테스트 가능
- DRM 감지: has_drm 비트 검사 추가, Hwp2MdError 타입으로 일관된 에러 반환

**검증**: cargo check 0 에러, clippy 0 경고, 142 테스트 (128 unit + 14 integration)

### 2026-04-22 — Phase 2: HWP 제어 문자 파싱 (14694df + 77db271)

**구현 내용**:
- HWP 5.0 제어 문자 파싱 6개 함수: find_children_end, extract_paragraphs_from_range, parse_table_ctrl, parse_gshape_ctrl, find_gsotype_bin_id, parse_ctrl_header_at
- 테이블: CTRL_TABLE + LIST_HEADER 레코드에서 행/열/셀 구조 추출, 셀→행 그룹핑
- 이미지: GSHAPE + GSOTYPE에서 BinData ID/크기 추출, IR→Markdown 이미지 블록 생성
- 각주/미주: CTRL_FOOTNOTE/ENDNOTE 본문 단락 추출
- OLE2 SummaryInformation 메타데이터 추출 (M7)
- BIN 스트림 최적화 (M9), --style 경고 (H4)

**리뷰 수정**:
- checked_mul/checked_add으로 32비트 정수 오버플로우 방지
- 행 인덱스 10,000 cap으로 할당 폭발 방지
- gshape/gsotype 테스트 5건 추가

**검증**: cargo check 0 에러, clippy 0 경고, 130 테스트 (116 unit + 14 integration)

### 2026-04-21 — 보안/정확도 수정 (99a1bc6)

**PM 리뷰**: 3개 병렬 진단 에이전트로 전체 코드 분석, 이슈 분류 (6 CRITICAL / 4 HIGH / 10 MEDIUM), 3개 병렬 수정 에이전트로 즉시 수정.

**수정 내용**:
- 보안 6건: 압축 폭탄(C1), 무제한 레코드(C2), 버퍼 오버런(C3), 서로게이트 검증(C4), 경로 순회(C5), ZIP-slip(C6)
- 정확도 3건: 에러 로깅(H1), 이미지 alt text(H6/H7), 한국어 글자수(H9)
- Clippy 3건: derive Default, range contains, collapsible if

**검증**: cargo check 0 에러, clippy 0 린트 (dead_code 17건 Phase 2 스캐폴딩)

### 2026-04-21 — Phase 1: 자체 구현 기반 재구성

**아키텍처 변경**: 라이선스 독립성을 위해 HWP 전용 크레이트(unhwp, hwpforge 등)를 모두 제거하고 자체 구현으로 전환.

**구현 내용**:
- `src/hwp/` — HWP 5.0 바이너리 파서
  - `model.rs`: FileHeader, DocInfo, CharShape, ParaShape, HwpParagraph, HwpControl
  - `record.rs`: 4-byte record header 파싱, tag constants, UTF-16LE 유틸
  - `reader.rs`: CFB 컨테이너 → zlib 해제 → record 해석 → IR 변환
- `src/hwpx/` — HWPX 파서/라이터
  - `reader.rs`: ZIP → XML 파싱 (header, section, table, image, equation)
  - `writer.rs`: IR → ZIP+XML (mimetype, container, version, header, section)
- `src/md/` — Markdown 파서/라이터
  - `writer.rs`: IR → GFM Markdown (frontmatter, table, HTML fallback)
  - `parser.rs`: comrak AST → IR (footnote, math, table, inline styles)
- `src/ir.rs` — 중간 표현 확장 (Math, underline, TableCell.blocks)
- `src/convert.rs` — 변환 오케스트레이터 (to-md, to-hwpx, info)
- `src/error.rs` — 에러 타입 확장 (Decompress, InvalidRecord, Encoding)

**의존성 변경**:
- 제거: unhwp, hwpforge, hwpforge-smithy-md, hwpforge-smithy-hwpx, hwpforge-core, hwpforge-foundation, pulldown-cmark, codepage, insta, serde_json
- 추가: serde_yaml
- 유지: cfb, flate2, zip, quick-xml, encoding_rs, byteorder, comrak, clap, serde, thiserror, anyhow, tracing

### 2026-04-21 — 프로젝트 초기화

- 기술 조사 완료 (Rust HWP 생태계, 기존 도구, 포맷 스펙)
- 프로젝트 scaffolding (이후 재구성됨)

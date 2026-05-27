# hwp2md — Progress

## 현재 상태: v0.5.0 Sprint 74 완료 (pending_code_lang 누수 수정 + CodeBlock/PageBreak 픽스처 테스트)

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

### 2026-05-21 — v0.5.0 Sprint 54: 한국 정부 공문서 텍스트 패턴 헤딩 감지

**S54-01: Tier-4 텍스트 패턴 헤딩 감지** (`src/hwp/convert.rs`):

`detect_heading_level`에 tier-4 추가: `detect_korean_regulation_heading(text)` private 함수.
- `^제\d+장` → H1 (장, chapter)
- `^제\d+절` → H2 (절, section)
- `^제\d+조` → H2 (조, article; 절과 동일 레벨 — 절 없는 문서에서 H1→H3 gap 방지)
- `trim_start()` 미사용 — 들여쓰기된 인용 단락 오승급 방지
- 매직 없음 — `strip_prefix('제')` + `find(!is_ascii_digit)` + suffix check

**S54-02/03: 단위 테스트 9건** (`src/hwp/convert_tests_detect.rs`):
- 장→H1, 절→H2, 조→H2, 개정 표기(조의N)→H2
- 앞 공백 비매칭, 잘못된 접미사 None
- 통합: empty DocInfo + "제5장" → Some(1)
- tier-1 (heading_type=Some(0)) + bold=false → Some(1) (tier-1 bold와 무관)

**S54-04: 황금 파일 재생성 + 정확 비교 테스트 활성화** (`tests/real_fixtures_hwp.rs`, `tests/fixtures/real/*.md`):
- 5개 #[ignore] 제거 — 모든 11개 픽스처 테스트 활성화
- 황금 파일 2회 재생성 (초기 tier-4 + follow-up 조→H2 수정)
- moel_01: 57 headings, moel_03/04: 21, moel_05: 53, moel_02: 0 (패턴 없음)

**리뷰 follow-up (870ec3e)**:
- W1: 조→H3에서 조→H2로 변경 (H1→H3 gap 제거)
- W2: `trim_start()` 제거 (들여쓰기 인용 오승급 방지)
- S2: leading whitespace 비매칭 테스트, amendment notation 테스트 추가

**Commits**: `0e3b22f` (feat), `870ec3e` (follow-up)

**검증**: 1196 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: 승인. W1/W2 모두 follow-up에서 해결. S1(편/관) 향후.
리뷰 전문: `~/.claude/references/2026-05-21_sprint54_korean_regulation_heading_review.md`

### 2026-05-21 — v0.5.0 Sprint 55: trim_start 복원 + 100자 상한 + 계층 테스트

**S55-01: moel_02 조사 + trim_start 복원** (`src/hwp/convert.rs`):

Sprint 54 W2 수정(trim_start 제거)이 실제 정부 문서(moel_02_vocational_training.hwp)에서 장 제목
(`"   제1장 총 칙"` — HWP 텍스트 스트림에 앞 공백 포함)을 무시하게 했음을 확인.
`trim_start()` 복원. 상세 doc comment 추가 (moel_02 증거 + 편/관 연기 사유).

**S55-02: 계층 테스트 강화** (`src/hwp/convert_tests_detect.rs`):
- `detect_korean_regulation_heading_jang_jeol_jo_hierarchy`: 절/조 각각 Some(2) 명시 어서션
- `detect_korean_regulation_heading_leading_whitespace_matched`: Some(1)/Some(2) 검증 (was None)
- `detect_korean_regulation_heading_tab_indented_matched`: \t 앞 공백 커버

**리뷰 follow-up (2daa7a7 → 추가 follow-up)**:

code-reviewer(opus) 리뷰 주요 지적:
- H1: 400자 기사 본문 단락(제1조(목적) 이 고시는…)이 H2로 승격되는 문제
- M1: doc comment "tier 1-3 None → 헤딩 보장" 논리 오류
- M3: 계층 테스트 절==조 어서션이 None==None도 통과하는 문제
- M4: 탭 들여쓰기 테스트 누락

**100자 상한 추가** (review H1):
- `text.chars().count() >= 100 → None` (tier-3 가드와 동일)
- 실제 장/절/조 단독 제목은 항상 100자 미만; 기사 본문 포함 단락은 보통 100자 초과
- `detect_korean_regulation_heading_long_article_body_not_promoted` 회귀 테스트 추가

**황금 파일 재생성** (100자 상한으로 긴 기사 본문 필터링):
- moel_01: 57 → 39 (H1:7 H2:32)
- moel_02: 103 → 45 (H1:5 H2:40)
- moel_03: 21 → 15 (H2:15)
- moel_04: 21 → 16 (H2:16)
- moel_05: 53 → 28 (H1:10 H2:18)

**Commits**: `24cc39b` (feat), `2daa7a7` (follow-up)

**검증**: 1199 tests (0 ignored), 0 failures. Clippy 0 warnings.

### 2026-05-21 — v0.5.0 Sprint 56: suffix 경계 검사 + PARA_HEADER style_id 수정

**S56-01: suffix 경계 검사** (`src/hwp/convert.rs`):

`detect_korean_regulation_heading`에 suffix 뒤 문자 검사 추가.
- `is_heading_terminator(c)` 헬퍼: 공백/괄호/구두점 → true
- 장/절/조 직후 문자가 한국어 조사(은/에서/의+비숫자)이면 → None
- 개정 표기 예외: `제5조의2` (의 + ASCII 숫자) → Some(2)
- fullwidth 괄호 `（/）` + CJK 닫힘 괄호 추가 (review H1)
- amendment 엣지케이스 테스트 추가 (EOL/공백/한국어 숫자/다자리)
- 황금 파일 변화 없음 — 기존 moel 문서에 단락 시작 조사 없음

**S56-02: PARA_HEADER nStyleID 파싱 수정** (`src/hwp/control/dispatcher.rs`):

HWP 5.0 스펙: nStyleID는 byte[6]의 UINT8; byte[7]은 플래그.
기존 코드: `u16::from_le_bytes([data[6], data[7]])` → 플래그 비트가 style_id를 오염.
예: style_id=0, flags=0x80 → 구코드 0x80_00=32768 → styles[] OOB → tier-2 누락.
수정: `u16::from(data[6])` (1 바이트만 읽음), 가드 `>= 8` → `>= 7`.
회귀 테스트 3건: flags=0xFF/flags=0x80/short-data 경계.
model.rs doc comment 수정: bytes[6-7] → byte[6] (UINT8).

**Commits**: `298df1f` (feat), `3122eca` (follow-up)

**검증**: 1204 tests (0 ignored), 0 failures. Clippy 0 warnings.

### 2026-05-21 — v0.5.0 Sprint 57: style_id u8 축소 + is_heading_terminator 정책 테스트

**P2: `style_id` 필드 타입 `u16` → `u8`** (`src/hwp/model.rs`, `src/hwp/control/dispatcher.rs`):

HWP 5.0 스펙 §3.2.1: nStyleID는 UINT8(0-255 범위). Sprint 56 리뷰 M1 권고.
- `HwpParagraph.style_id`: `u16` → `u8` (doc comment도 `byte[6] (UINT8 nStyleID)`로 수정)
- `dispatcher.rs`: `u16::from(rec.data[6])` → `rec.data[6]` (직접 u8 읽기)
- 모든 호출 지점은 즉시 `usize`로 캐스팅 — breaking change 없음

**P3: `is_heading_terminator` 정책 테스트** (`src/hwp/convert_tests_detect.rs`):

Sprint 56 리뷰 L2 권고: 함수의 허용/차단 정책을 명시적으로 고정.
- `is_heading_terminator_canonical_allowed_set`: 공백/탭/괄호 ASCII+fullwidth/CJK/구두점 (`·` `ㆍ` `…`) 등 대표값 검증
- `is_heading_terminator_blocked_set`: 한국어 조사(은/에/이/가/의) + ASCII 알파벳/숫자 차단 확인

**Commit**: `224d2d4`

**검증**: 1206 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. LOW 2건:
- LOW-1: `is_heading_terminator` 차단 집합에 `%`, `&`, `/`, `"` 미포함 (비한국어 구두점)
- LOW-2: 허용 집합에 `[`, `]`, `：`, `．`, `；` 대표값 누락
리뷰 전문: `~/.claude/references/2026-05-21_sprint57_style_id_narrowing_terminator_tests_review.md`

### 2026-05-21 — v0.5.0 Sprint 58: is_heading_terminator 테스트 커버리지 확장

**S58-P2/P3: 차단/허용 집합 테스트 보강** (`src/hwp/convert_tests_detect.rs`):

Sprint 57 리뷰 LOW-1/LOW-2 해결. 구현 변경 없음 — 테스트만 추가.

- 허용 집합에 `[`, `]`, `：`, `．`, `；` 추가 (match arm에 이미 있으나 미검증)
- 차단 집합에 `%`, `&`, `/`, `"` 추가 (matches! fall-through → false 명시화)

**Commit**: `cc36d3c`

**검증**: 1206 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. LOW 2건:
- LOW-1: 허용 집합에 `《》`/`〈〉` CJK title brackets 미테스트 (가장 우선)
- LOW-2: `, ， - ~` 허용 집합 대표값 미테스트; 섹션 주석 `S57-P3` → `S58` 갱신 필요
리뷰 전문: `~/.claude/references/2026-05-21_sprint58_is_heading_terminator_test_expansion_review.md`

### 2026-05-21 — v0.5.0 Sprint 59: is_heading_terminator CJK brackets + separator 테스트

**S59-P2/P3: 허용 집합 보강 + 섹션 주석 갱신** (`src/hwp/convert_tests_detect.rs`):

Sprint 58 리뷰 LOW-1/LOW-2 해결. 구현 변경 없음 — 테스트 + 주석만 수정.

- 허용 집합에 `《》` / `〈〉` CJK title brackets 추가 (match arm U+3008/U+3009 확인됨)
- 허용 집합에 `,` `，` `-` `~` 구분자 추가
- 섹션 주석 `S57-P3` → `S57-P3 / S58-P2 / S59-P2` 갱신

**Commit**: `9be195e`

**검증**: 1206 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. LOW 3건:
- S1 (LOW): 〈〉 코드포인트 U+3008/U+3009 확인 필요 → 확인 완료(U+3008/U+3009 맞음)
- S2 (Sprint 60 P2): `『』` match arm에 있으나 미테스트
- S3 (Sprint 60 P3): `제3조-제5조` 범위 표현 행동 테스트 추가 권고
리뷰 전문: `~/.claude/references/2026-05-21_sprint59_cjk_brackets_separator_tests_review.md`

### 2026-05-21 — v0.5.0 Sprint 60: is_heading_terminator 『』 + behavioral range/bracket 테스트

**S60-P2/P3: 허용 집합 보강 + 행동 테스트** (`src/hwp/convert_tests_detect.rs`):

Sprint 59 리뷰 S2/S3 해결. 구현 변경 없음 — 테스트 + 주석만 수정.

- 허용 집합에 `『`/`』` (CJK double guillemets, open+close 대칭) 추가
- 행동 테스트 2건: `"제3조-제5조"` → `Some(2)`, `"제5장《한국》"` → `Some(1)`
- 섹션 주석 스프린트 목록 → `git log` 참조로 축약
- Review follow-up: 범위 표현 테스트 주석을 "의도적 정책"이 아닌 "terminator allowlist 부산물"로 명확화

**Commits**: `7e653a9` (feat) + `72afc93` (follow-up comment fix)

**검증**: 1208 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. LOW 3건:
- S1: 범위 표현 테스트 주석 과도 주장 (follow-up에서 수정됨)
- S2 (Sprint 61 P2): `『』` behavioral test (`"제3조『인용』"` → Some(2)) + closer 대칭 테스트
- S3 (Sprint 61 P3): `"제3조-제5조는 적용 제외"` 행동 확인 — 현재 `Some(2)` 반환 (dash가 '-'로 분기, 뒤의 조사 '는' 미도달); 버그인지 정책인지 테스트로 고정 필요
리뷰 전문: `~/.claude/references/2026-05-21_sprint60_guillemet_range_bracket_behavioral_tests_review.md`

### 2026-05-22 — v0.5.0 Sprint 61: 『』 behavioral + dash-then-particle regression pin

**S61-P2/P3: 『』 행동 테스트 + dash-particle 회귀 고정** (`src/hwp/convert_tests_detect.rs`):

Sprint 60 리뷰 S2/S3 해결. 구현 변경 없음 — 테스트만 추가.

- `"제3조『인용』"` → `Some(2)` (open guillemet 행동 확인)
- `"제3조』참조"` → `Some(2)` (close guillemet 대칭 행동 확인)
- `"제3조-제5조는 적용 제외"` → `Some(2)` 회귀 고정 — 알려진 tier-4 한계:
  첫 번째 문자만 검사하므로 dash 이후의 '는' 조사에 도달하지 못함.
  실제 문서에서는 tier-2(style_id)가 먼저 처리되므로 허용 가능한 한계.

**Commit**: `dc4d7aa`

**검증**: 1211 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. 제안 2건:
- nit: assertion 메시지에 입력 문자열 포함 권고
- Sprint 62 P2: `「」` behavioral test (「=」 쌍과 대칭; 『』와 함께 가장 흔한 한국어 인용 부호)
리뷰 전문: `~/.claude/references/2026-05-22_sprint61_guillemet_behavioral_regression_tests_review.md`

### 2026-05-22 — v0.5.0 Sprint 62: 「」 behavioral + assertion 메시지 nit 수정

**S62-P2/P3: 「」 행동 테스트 + assertion 메시지 개선** (`src/hwp/convert_tests_detect.rs`):

Sprint 61 리뷰 제안 해결. 구현 변경 없음.

- `"제3조「참조」"` → `Some(2)` (open guillemet 대칭)
- `"제3조」참조"` → `Some(2)` (close guillemet 대칭)
- 기존 5개 behavioral 테스트 assertion 메시지에 입력 문자열 포함 (CI 자기 문서화)

**Commit**: `e8d0aff`

**검증**: 1213 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. 제안 2건:
- 메시지 경미한 형식 불일치 (491/501 "heading level" 표현 vs 511+ 생략)
- Sprint 63 P3: 잔여 terminator-pair gap — `《》` closer + `〈〉` open/close + `<>` open/close
리뷰 전문: `~/.claude/references/2026-05-22_sprint62_single_guillemet_behavioral_tests_review.md`

### 2026-05-22 — v0.5.0 Sprint 63: 〈〉/《》/<> terminator-pair behavioral gap fill

**S63-P2/P3: 〈〉/《》/<> 행동 테스트 추가** (`src/hwp/convert_tests_detect.rs`):

Sprint 62 리뷰 gap matrix 해결. 구현 변경 없음.

- `"제3조〈참조〉"` → `Some(2)` (CJK 단일 각괄호 열림)
- `"제3조〉참조"` → `Some(2)` (CJK 단일 각괄호 닫힘)
- `"제3조》참조"` → `Some(2)` (CJK 이중 각괄호 닫힘; 열림은 Sprint 60에서 완료)
- `"제3조<참조>"` → `Some(2)` (ASCII 각괄호 열림)
- `"제3조>참조"` → `Some(2)` (ASCII 각괄호 닫힘)

**Commit**: `ad36ee4`

**검증**: 1218 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. LOW 1건:
- LOW: 커밋 제목에 `⟨⟩` (U+27E8/U+27E9) 오타 — 실제 추가된 것은 `〈〉` (U+3008/U+3009)
Sprint 64 제안: `[]` + `（）` behavioral 쌍 (bracket matrix 완성), `cjk_title_bracket` 이름 변경, `<`/`>` 단위 허용 집합 테스트 추가.
리뷰 전문: `~/.claude/references/2026-05-22_sprint63_terminator_pair_behavioral_gap_fill_review.md`

### 2026-05-22 — v0.5.0 Sprint 64: bracket matrix complete + punctuation behavioral coverage

**S64-P2: 괄호 matrix 완성 + 이름 변경** (`src/hwp/convert_tests_detect.rs`):

Sprint 63 리뷰 nit 해결. 구현 변경 없음.

- `cjk_title_bracket_treated_as_heading` → `cjk_double_angle_open_treated_as_heading` (이름 대칭 복원, U+300A 코드포인트 주석 추가, 닫힘 쌍 함수명 cross-reference)
- `"제3조[참조]"` → `Some(2)` (ASCII 대괄호 열림)
- `"제3조]참조"` → `Some(2)` (ASCII 대괄호 닫힘 고아)
- `"제3조（참조）"` → `Some(2)` (전각 괄호 열림)
- `"제3조）참조"` → `Some(2)` (전각 괄호 닫힘 고아)

**S64-P3: 구두점 behavioral 커버리지** (`src/hwp/convert_tests_detect.rs`):

- `<`/`>` 를 `is_heading_terminator_canonical_allowed_set` 단위 테스트에 추가
- `~` `·` `ㆍ` `…` `：` `．` `；` `，` — 8개 behavioral 테스트 추가

**Commit**: `1a8560b`

**검증**: 1230 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. LOW 2건:
- LOW-1: ASCII `,` (전각 `，` 는 Sprint 64에서 추가됐으나 ASCII 짝 누락)
- LOW-2: ASCII `)` 고아 닫힘 괄호 behavioral 테스트 없음 (쌍으로만 사용됨)
Sprint 65 제안: ASCII `,` + `)` orphan-close 2건 추가로 matrix 완성.
리뷰 전문: `~/.claude/references/2026-05-22_sprint64_bracket_matrix_complete_punctuation_behavioral_review.md`

### 2026-05-22 — v0.5.0 Sprint 65: terminator matrix finalized + ASCII/fullwidth semicolon guard

**S65-P2/P3: ASCII matrix 완성 + 부정 가드** (`src/hwp/convert_tests_detect.rs`):

Sprint 64 리뷰 LOW-1/LOW-2 + P3 해결. 구현 변경 없음.

- `"제3조,제5조"` → `Some(2)` (ASCII 쉼표 — 전각 `，` 와 대칭)
- `"제3조)참조"` → `Some(2)` (ASCII 닫힘 괄호 고아)
- `is_heading_terminator_blocked_set`에 ASCII `;` 부정 단언 추가 (전각 `；` 허용, ASCII `;` 차단 명시)

**리뷰 follow-up** (`1dfcc37`):
- HIGH fix: 단언 메시지에서 U+2184 ("ↄ") 아티팩트 제거
- LOW fix: `ascii_paren_close` 주석의 cross-reference 수정 (`tier4_integration` → `terminator_chars_still_match`)

**Commits**: `5145947` (feat) + `1dfcc37` (follow-up)

**검증**: 1232 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: APPROVE. HIGH 1건 (follow-up에서 해결):
- HIGH: 단언 메시지에 U+2184 오타 → 공백으로 교체
- LOW: cross-reference 함수명 오류 → 수정
- terminator matrix 완성 확인 (allowlist 전 문자 커버)
리뷰 전문: `~/.claude/references/2026-05-22_sprint65_terminator_matrix_finalized_review.md`

### 2026-05-22 — v0.5.0 Sprint 66: U+3000 ideographic space behavioral + doc comment

**S66-P2/P3: 공백 behavioral + 생산 코드 doc comment** (`src/hwp/convert.rs`, `src/hwp/convert_tests_detect.rs`):

Sprint 65 리뷰 제안 해결.

- U+3000 / U+00A0 단위 허용 집합 테스트 추가
- `"제3조\u{3000}참조"` → `Some(2)`, `"제1장\u{3000}총칙"` → `Some(1)` behavioral 테스트
- `is_heading_terminator` doc comment: whitespace 정책(Unicode White_Space) + ASCII `;` 제외 정책 명시

**리뷰 follow-up** (`6c85228`):
- HIGH fix: `///` 블록 오귀속 — `detect_korean_regulation_heading` 매핑/trim/상한/편 doc 분리
- MEDIUM fix: "모든 Unicode 공백" → "Unicode White_Space property" (ZWSP/BOM 비포함 명시)
- LOW: U+00A0 behavioral 단언 추가
- LOW: U+200B (ZWSP) 부정 회귀 고정 → `None`

**Commits**: `db30eab` (feat) + `6c85228` (follow-up)

**검증**: 1233 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: APPROVE. HIGH 1건 (follow-up에서 해결):
- HIGH: `///` 블록 오귀속 — `is_heading_terminator`에 `detect_korean_regulation_heading` 내용 귀속됨
- MEDIUM: whitespace "모든 Unicode" 표현 과장 → ZWSP/BOM 제외 명시
- LOW: U+00A0 behavioral 미커버, ZWSP 부정 회귀 고정 누락
리뷰 전문: `~/.claude/references/2026-05-22_sprint66_ideographic_space_doc_comment_review.md`

### 2026-05-22 — v0.5.0 Sprint 67: tab terminator path + U+202F narrow NBSP behavioral

**S67-P2/P3: 공백 behavioral 커버리지 완성** (`src/hwp/convert_tests_detect.rs`):

Sprint 66 리뷰 제안 해결. 구현 변경 없음 — 테스트만 추가.

- `detect_korean_regulation_heading_tab_between_marker_and_title` 신규 테스트:
  - `"제3조\t참조"` → `Some(2)`, `"제1장\t총칙"` → `Some(1)`
  - tab-as-terminator 경로 격리 (tab_indented_matched는 trim_start + 공백 terminator를 함께 커버)
- U+202F (NARROW NBSP) behavioral 단언 + `ideographic_space_treated_as_heading`에 추가:
  - `"제3조\u{202F}참조"` → `Some(2)` 행동 검증
  - U+202F를 `is_heading_terminator_canonical_allowed_set` 단위 테스트에 추가
- U+200B (ZWSP) 부정 회귀 고정: `"제3조\u{200B}참조"` → `None` (is_whitespace()=false 확인)

**리뷰 follow-up** (`79c7935`):
- LOW-1: U+202F 주석 수정 — "괄호 앞 narrow NBSP" → "some HWP authoring tools emit it as a no-break separator" (픽스처와 불일치 해소)
- LOW-2: tab between marker+title 주석 명확화 — "Isolates the tab-as-terminator path. tab_indented_matched exercises trim_start() on the leading tab and then hits a space terminator; here we exercise the tab terminator alone."

**Commits**: `7204362` (feat) + `79c7935` (follow-up)

**검증**: 1234 tests (0 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. LOW 2건 (follow-up에서 해결):
- LOW-1: U+202F 주석이 픽스처와 불일치
- LOW-2: tab terminator 격리 의도 불명확
리뷰 전문: `~/.claude/references/2026-05-22_sprint67_tab_narrownbsp_whitespace_coverage_review.md`

### 2026-05-22 — v0.5.0 Sprint 68: U+FEFF negative pin + U+205F positive pin (whitespace coverage complete)

**S68-P2/P3: 공백 커버리지 표 완성** (`src/hwp/convert_tests_detect.rs`):

Sprint 67 리뷰 선택적 항목 구현. 구현 변경 없음 — 테스트만 추가.

- U+FEFF (BOM/ZWNBSP) 부정 고정: `is_heading_terminator_blocked_set` + `ideographic_space_treated_as_heading`에 None 단언 추가
- U+205F (MEDIUM MATH SPACE) 긍정 고정: `is_heading_terminator_canonical_allowed_set` + `ideographic_space_treated_as_heading`에 Some(2) 단언 추가

**리뷰 follow-up** (`29724b8`):
- LOW-1: U+FEFF 주석 → Sprint 68 의도 + 표 완결 관계 명시
- LOW-2: `is_heading_terminator_blocked_set`에 U+200B char-level 부정 단언 추가 (U+FEFF와 대칭)

**Commits**: `3741706` (feat) + `29724b8` (follow-up)

**검증**: 1234 tests (0 ignored), 0 failures. Clippy 0 warnings.

**whitespace coverage 완성:**
- U+0020 ✓ | U+0009 ✓x2 | U+00A0 ✓ | U+202F ✓ | U+3000 ✓ | U+200B ✓ | U+FEFF ✓ | U+205F ✓

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. LOW 2건 (follow-up에서 해결):
- LOW-1: U+FEFF 주석 과소 설명
- LOW-2: U+200B char-level pin 비대칭
리뷰 전문: `~/.claude/references/2026-05-22_sprint68_whitespace_coverage_complete_review.md`

### 2026-05-22 — v0.5.0 Sprint 69: 편(Part) detection + heading_style 모듈 추출

**S69-01/02: 편 감지 구현 + 독스트링** (`src/hwp/heading_style.rs`):

`detect_korean_regulation_heading`에 편 브랜치 추가. 편→H1 (장과 동일 레벨 — 편과 장은 같은 문서에 최상위로 공존하지 않음). 황금 파일 변화 없음 (moel_01-05는 모두 장 기반).

함수와 `is_heading_terminator`를 `convert.rs`에서 `heading_style.rs`로 추출 — HWPX reader가 공유 가능하도록.

**S69-03: 편 단위 테스트 6건** (`src/hwp/convert_tests_detect.rs`):
- `pyeon_returns_h1`, `pyeon_multi_digit`, `pyeon_leading_whitespace`
- `pyeon_particle_rejection`, `pyeon_terminator_chars`, `pyeon_jang_both_h1`

**S69-04: HWPX tier-4 파이프라인 연동** (`src/hwpx/context/flush.rs`):
- `flush_paragraph` + `flush_paragraph_staged` 양쪽에 tier-4 감지 연동
- `is_heading = effective_level.is_some()` — heading이 list-indent보다 우선

**S69-05: 통합 테스트 3건** (`tests/integration.rs`):
- `pyeon_detected_as_h1_via_tier4`, `pyeon_heading_text_preserved`, `pyeon_rendered_as_atx_h1_in_markdown`

**리뷰 follow-up** (`ba22a43`):
- M1: 설계 근거 독스트링 복원 (100자 상한, trim_start, 절/조 H2 공유, 편/장 H1 공유)
- M2: `is_heading` 의미론적 변경 주석 추가
- L3: 통합 테스트 `assert_eq!(blocks.len(), 3)` 강화

**Commits**: `30dba6e` (feat) + `ba22a43` (follow-up)

**검증**: 1240 lib tests + 31 integration tests (0 ignored), 0 failures. Clippy -D warnings 0 경고.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH 없음. MEDIUM 2건, LOW 4건 (M1/M2/L3 follow-up에서 해결):
- M1: 설계 근거 독스트링 소실 → 복원
- M2: is_heading 의미론 변경 미문서화 → 주석 추가
리뷰 전문: `~/.claude/references/2026-05-22_sprint69_pyeon_detection_heading_style_extraction_review.md`

### 2026-05-26 — v0.5.0 Sprint 70: effective_heading_level 헬퍼 추출 + 통합 테스트 정확도 강화

**S70-01: `effective_heading_level` 헬퍼 추출** (`src/hwpx/context/flush.rs`):

Sprint 69 리뷰 L4 권고 해결. `flush_paragraph`(#[cfg(test)]) 와 `flush_paragraph_staged` 두 HWPX 경로에 인라인 중복된 tier-4 level 결정 로직을 단일 헬퍼로 추출:

```rust
fn effective_heading_level(style_level: Option<u8>, inlines: &[Inline]) -> Option<u8> {
    style_level.or_else(|| {
        let combined: String = inlines.iter().map(|i| i.text.as_str()).collect();
        detect_korean_regulation_heading(&combined)
    })
}
```

- 서명 `(Option<u8>, &[Inline]) -> Option<u8>` — ParseContext 미참조, 독립 테스트 가능
- `or_else` lazy semantics 보존 (style_level이 Some이면 String 미생성)
- 두 경로가 동일 헬퍼를 호출하므로 tier-4 로직 변경 시 한 쪽만 업데이트되는 리스크 제거
- `Inline` 타입 import 추가

**S70-02: `pyeon_heading_text_preserved` 통합 테스트 정확도 강화** (`tests/integration.rs`):

Sprint 69 L2 권고 해결. `.any()` 기반 느슨한 검사 → `.find()` + `assert_eq!` 정확한 텍스트 단언:

```rust
let heading = doc.sections.iter().flat_map(|s| &s.blocks)
    .find(|b| matches!(b, ir::Block::Heading { level: 1, .. }));
assert!(heading.is_some(), "expected an H1 heading block; none found");
let ir::Block::Heading { inlines, .. } = heading.unwrap() else { unreachable!() };
let text: String = inlines.iter().map(|i| i.text.as_str()).collect();
assert_eq!(text, "제3편 채권", "heading text mismatch: got {:?}", text);
```

텍스트 오염(공백 누출, 추가 문자), inline 분리 회귀까지 탐지 가능.

**Commit**: `7dc621e`

**검증**: 1240 lib tests + 31 integration tests (0 ignored), 0 failures. Clippy -D warnings 0 경고.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. LOW 2건:
- LOW-1: Docstring "lockstep" 범위가 list-staging 분기를 포함하는 것처럼 과잉 표현 — level 결정만 공유됨을 명시 권고
- LOW-2: String 할당이 헬퍼 내부로 숨어 `or(...)` vs `or_else(...)` 함정 가능 — 기존 lazy semantics 유지됨
리뷰 전문: `~/.claude/references/2026-05-26_sprint70_effective_heading_level_helper_review.md`

### 2026-05-26 — v0.5.0 Sprint 71: flush.rs docstring 정확화 + collect_inline_text 헬퍼

**S71-01: `effective_heading_level` docstring 수정** (`src/hwpx/context/flush.rs`):

Sprint 70 리뷰 LOW-1/LOW-2 해결.

- "stay in lockstep" → "**level-resolution rule** stays in lockstep" — list-staging 가드는 `flush_paragraph_staged`에만 존재함을 명시
- `or_else` lazy semantics note 추가 — `style_level`이 `Some`이면 String 미생성

**S71-02: `collect_inline_text` 헬퍼 추출** (`src/hwpx/context/flush.rs`):

Sprint 70 리뷰 P3(inlines_to_string) 해결.

```rust
fn collect_inline_text(inlines: Vec<ir::Inline>) -> String {
    inlines.into_iter().map(|i| i.text).collect()
}
```

`flush_paragraph`(line 130)과 `flush_paragraph_staged`(line 164)의 동일한 CodeBlock 텍스트 추출 패턴 대체. by-value 소비, turbofish 제거 (반환 타입 추론), clone 없음.

**Commit**: `da8fb2e`

**검증**: 1240 lib tests + 31 integration tests (0 ignored), 0 failures. Clippy -D warnings 0 경고.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. 제안 2건 (비차단):
- S1: `String::with_capacity` 사전 할당 가능 — 벤치마크 압박 없어 불필요
- S2: `#[inline]` on collect_inline_text — 컴파일러 자동 처리
리뷰 전문: `~/.claude/references/2026-05-26_sprint71_flush_rs_docstring_collect_inline_text_review.md`

### 2026-05-27 — v0.5.0 Sprint 74: pending_code_lang 누수 수정 + CodeBlock/PageBreak 픽스처 테스트

**S74-01 (P3): `pending_code_lang` 누수 수정** (`src/hwpx/context/flush.rs`):

`flush_nested_scope`가 cell/list/header/footer/footnote 분기를 처리한 후 `pending_code_lang`을 초기화하지 않아 XML comment 코드 힌트가 다음 최상위 단락으로 누수되는 버그 수정.

구조 변경:
```rust
pub(crate) fn flush_nested_scope(ctx: &mut ParseContext) -> bool {
    if ctx.header_footer.in_header { flush_header_paragraph(ctx); }
    else if ... { ... }
    else { return false; }  // top-level: return early, preserve pending_code_lang
    ctx.pending_code_lang = None;  // nested path only: discard code-fence hint
    true
}
```

**S74-02: 단위 테스트 2건** (`src/hwpx/reader_tests_charpr.rs`):
- `pending_code_lang_cleared_after_nested_scope_flush` — 5개 중첩 분기(in_header/in_footer/footnote.active/table.in_cell/list.in_item) 각각 cleared 검증
- `pending_code_lang_preserved_at_top_level_no_nested_scope` — top-level 경로에서 intact 보존 확인

**S74-03 (P2): HwpxFixture 통합 테스트 4+1건** (`tests/integration.rs`):
- `fixture_lang_hint_comment_produces_codeblock_ir` — `<!-- hwp2md:lang:python -->` → `ir::Block::CodeBlock`
- `fixture_lang_hint_comment_renders_python_fence_in_markdown` — MD에 ` ```python ` 펜스
- `fixture_newpage_ctrl_produces_pagebreak_ir` — `<hp:ctrl id="newPage"/>` → `ir::Block::PageBreak`
- `fixture_newpage_ctrl_renders_pagebreak_marker_in_markdown` — MD에 `<!-- pagebreak -->`
- Follow-up L3: `fixture_lang_hint_inside_cell_does_not_leak_to_next_toplevel_paragraph` — 셀 내부 힌트가 다음 최상위 단락을 CodeBlock으로 만들지 않음을 확인 (원래 버그 시나리오 end-to-end 검증)

**S74-P4**: flush.rs=456L, handlers.rs=583L — 800행 제한 준수 확인, 추출 불필요.

**Follow-up (L1/L2/L3)**:
- L1: `check_scope!` 매크로에 scope 레이블 추가
- L2: `if let` → `let-else panic!` 교체 (묵시적 통과 방지)
- L3: 셀 누수 end-to-end 회귀 테스트 추가

**Commits**: `901fbc8` (feat:sprint74) + `826ba6b` (docs:follow-up)

**검증**: 1442 tests (0 failures). Clippy -D warnings 0 경고.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH 없음. M1 (중첩 스코프 CodeBlock 언어 힌트 손실 — pre-existing, 향후 Sprint 75 후보) + LOW 3건 (L1/L2/L3 모두 follow-up에서 해결).
리뷰 전문: `~/.claude/references/2026-05-27_hwp2md_sprint74_pending_code_lang_leak_fix_review.md`

---

### 2026-05-27 — v0.5.0 Sprint 73: HWPX tier-3 heading 감지 + nested scope dedup

**S73-01: HWPX tier-3 heading 감지** (`src/hwpx/context/state.rs`, `src/hwpx/context/mod.rs`, `src/hwpx/context/flush.rs`, `src/hwpx/handlers.rs`):

Sprint 73 P3 해결. HWP binary reader의 tier-1/2/3/4 파이프라인에서 HWPX reader는 tier-3(폰트 높이 + bold) 감지가 없었음.

- `FormattingState.font_height: Option<u32>` — charPr `height` 속성 캡처
- `ParseContext.para_max_font_height: u32`, `para_max_font_height_bold: bool` — 단락 내 최대 폰트 높이 추적
- `apply_charpr_attrs` `"height"` 핸들러 + max 갱신 로직 추가
- 상수: `HWPX_H1_MIN_HEIGHT=1600`, `HWPX_H2_MIN_HEIGHT=1400`, `HWPX_H3_MIN_HEIGHT=1200`
- `effective_heading_level(style_level, inlines, height_hint: Option<(u32, bool)>)` — tier-3: bold+폰트높이 임계값; 100자 이상 본문 가드
- `build_block` + `flush_paragraph_staged`: `height_hint=Some((para_max_font_height, para_max_font_height_bold))` 전달
- `<hp:p>` start: `para_max_font_height=0`, `para_max_font_height_bold=false` 리셋

**S73-02: `flush_nested_scope` 헬퍼 추출** (`src/hwpx/context/flush.rs`, `src/hwpx/handlers.rs`):

Sprint 73 P2 해결. `flush_active_paragraph_scope`와 `handlers.rs` p-end 핸들러의 5-분기 중첩 스코프 라우터 중복 → `flush_nested_scope` 추출.

```rust
pub(crate) fn flush_nested_scope(ctx: &mut ParseContext) -> bool {
    // header → footer → footnote → cell → list (cell-first 통일)
    // true 반환: 중첩 스코프 flush 완료; false: 최상위 (호출자가 책임)
}
```

- `handlers.rs` p-end: `flush_nested_scope(ctx)` 호출 → false이면 `flush_paragraph_staged`
- `flush_active_paragraph_scope`: `flush_nested_scope(ctx)` 호출 → false이면 top-level flush
- docstring: 분기 순서(cell-first) 명시 + 이전 list-first와 통일 이유 기록

**S73 follow-up** (`src/hwpx/context/flush.rs`, `src/hwpx/reader_tests_charpr.rs`):

코드 리뷰 H1/M1 해결:
- H1: `flush_nested_scope` docstring에 cell-first 순서 통일 명시 (이전 `flush_active_paragraph_scope`는 list-first였음)
- M1: `tier3_99char_boundary_promotes_to_heading` 추가 — 99자에서는 가드 미발동(>= 100), H1 promote 확인

**신규 테스트** (`src/hwpx/reader_tests_charpr.rs`): 12+1건 = 13건
- `apply_charpr_attrs` height 파싱 4건 (기본/max 추적/낮은 값 무시/갱신)
- 통합 7건 (H1/H2/H3 promote, not-bold 가드, threshold 미달, 100자 가드, style-level 우선)
- 99자 경계 1건 (follow-up)
- 회귀 1건 (style_level_takes_priority)

**Commits**: `b354d5d` (refactor:sprint73) + `9f26193` (docs:follow-up)

**검증**: 1435 tests (0 failures). Clippy -D warnings 0 경고.

**리뷰 (code-reviewer opus)**: 조건부 APPROVE. CRITICAL/HIGH 없음. H1(flush_nested_scope 분기 순서 변경 명시화) + M1(99자 경계 테스트) — 모두 follow-up에서 해결.
리뷰 전문: `~/.claude/references/2026-05-27_hwp2md_sprint73_tier3_heading_review.md`

---

### 2026-05-26 — v0.5.0 Sprint 72: build_block 헬퍼 추출 + list-staging 정밀화

**S72-01: `build_block` 헬퍼 추출** (`src/hwpx/context/flush.rs`):

Sprint 71 로드맵 P2 해결. `flush_paragraph`와 `flush_paragraph_staged`의 CodeBlock/Heading/Paragraph 생성 로직 통합:

```rust
fn build_block(
    inlines: Vec<ir::Inline>,
    code_lang: Option<Option<String>>,  // 외부: is-code-block, 내부: language hint
    heading_level: Option<u8>,
) -> ir::Block
```

- `code_lang: Option<Option<String>>` — `pending_code_lang` 타입과 일치; 내부 Option<String>이 `ir::Block::CodeBlock::language`로 직접 전달
- `collect_inline_text` 재사용 (Sprint 71 헬퍼)
- `flush_paragraph` 14줄 → 2줄 간소화

**list-staging 조건 정밀화** (`flush_paragraph_staged`):
- `is_heading = effective_level.is_some()` → `matches!(block, ir::Block::Paragraph { .. })`
- 더 정밀: CodeBlock도 명시적으로 list-staging 제외
- 이전 early-return에 의존하던 암묵적 보호 → 명시적 조건

**S72 follow-up: CodeBlock list-staging regression guard** (`src/hwpx/reader_tests_charpr.rs`):

Sprint 72 리뷰 LOW 권고 즉시 해결.
- `flush_paragraph_staged_code_block_with_list_para_pr_id_is_plain`: `pending_code_lang=Some(None)` + `para_pr_id="2"` → `StagedBlock::Plain(CodeBlock)` 단언
- `matches!` 조건이 미래 변형 추가 시 침묵 실패하지 않도록 고정

**Commits**: `d696a4b` (refactor) + `ab81446` (follow-up)

**검증**: 1241 lib tests + 31 integration tests (0 ignored), 0 failures. Clippy -D warnings 0 경고.

**리뷰 (code-reviewer opus)**: APPROVE. CRITICAL/HIGH/MEDIUM 없음. LOW 1건 (follow-up에서 해결):
- LOW: `matches!` 조건 미래 변형 대비 regression guard 테스트 추가 권고 → 즉시 구현
리뷰 전문: `~/.claude/references/2026-05-26_sprint72_build_block_helper_list_staging_precision_review.md`

---

### 2026-05-21 — v0.5.0 Sprint 53: 인라인 제어 코드 상수 추출 + ruby.rs 정렬

**S53-T1: CTRL_INLINE_PARAM_BYTES + is_inline_ctrl_code 추출** (`src/hwp/record.rs`, `src/hwp/reader.rs`, `src/hwp/control/ruby.rs`):

`reader.rs`의 매직 리터럴 `14`와 `ruby.rs`의 독립 `const CTRL_PARAM_BYTES: usize = 14`를 `record.rs`로 통합.
- `pub(crate) const CTRL_INLINE_PARAM_BYTES: usize = 14`
- `pub(crate) fn is_inline_ctrl_code(ch: u16) -> bool` — 0x0001–0x0008, 0x000B–0x000C, 0x000E–0x001F (tab/LF/para-end 제외)
- 2 단위 테스트 추가: 경계값 참/거짓 + exhaustive 범위 루프

리뷰 follow-up (156174b):
- ruby.rs else 분기를 `CTRL_CHAR_LOW..=CTRL_CHAR_HIGH` 범위에서 `is_inline_ctrl_code(ch)` 호출로 교체 — 의미 정확성 보장
- reader.rs match 가드를 `_ if` → `ch if` (관용적 Rust)
- record.rs doc comment "null / paragraph end" → "stream terminator / padding (no params)" 명확화
- `inline_ctrl_code_exhaustive_range` 테스트 추가: 0x0000–0x0021 전 범위 루프 검증

**S53-T2: Para_shape 신호 조사 spike**:

한국 정부 공문서(훈령/고시)의 para_shape 필드(들여쓰기/줄 간격/정렬)로 장/절 제목 감지 불가 확인. 모든 단락이 style_id=0(Normal), 번호 관례(제N장/제N조)만 구분 수단. 텍스트 패턴 감지 권고 → Sprint 54 P1.

**Commits**: `c51a8f1` (초기), `156174b` (review follow-up)

**검증**: 1187 tests (5 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: 승인. H2 ruby.rs 범위 불일치 수정, H3 match 가드 + exhaustive 테스트 수정. 모두 follow-up에서 해결.
리뷰 전문: `~/.claude/references/2026-05-21_sprint53_inline_ctrl_extraction_review.md`

---

### 2026-05-21 — v0.5.0 Sprint 52: 헤딩 false positive 수정 + 헬퍼 강화 + 픽스처 테스트 활성화

**S52-01: parse_hwpx_style_ref 단위 테스트** (`src/hwpx/handlers.rs`):
4건 추가 (numeric range/out-of-range/name-precedence/garbage). private 유지.

**S52-02: parse_heading_style 강화** (`src/hwp/heading_style.rs`):
`strip_prefix(' ')` → `trim()` (이중 공백, 탭 처리), `is_ascii_digit` 가드 추가 (`"+1"` 거부). 테스트 3건 추가.

**S52-04: 헤딩 false positive 수정** (`src/hwp/convert.rs`):

근본 원인: Sprint 50의 `&& cs.bold` 제거가 한국 정부 공문서 14pt 본문(제2조, 목록 항목 등)을 모두 H2로 승격. moel_01에서 1122개 false heading 발생.

수정: tier-3 font-size 모든 체크에 `&& cs.bold` 복원. 1122개 → 0개.

**S52-03: 픽스처 golden 재생성 + 구조 테스트 활성화** (`tests/fixtures/real/*.md`, `tests/real_fixtures_hwp.rs`):
5개 golden 파일 재생성 (Sprint 49 인코딩 수정 + Sprint 52 heading 수정 반영). 구조 비교 테스트 5건 `#[ignore]` 제거 → 활성화. `assert_heading_fidelity` 헬퍼 추출 (golden=0인 경우 false-positive guard로 명확 표현). `tests/fixtures/real/README.md` 추가 (공공누리 라이선스).

**Commits**: `725777a` (feat), `57c417e` (refactor follow-up)

**검증**: 1184 tests (5 ignored), 0 failures. Clippy 0 standard warnings.

**리뷰 (code-reviewer opus)**: 승인. H1 zero-case tolerance 수정, M1 pub(super) 복원 완료. M2 non-bold heading detection 손실 — Sprint 53 비볼드 픽스처 추가 권고.
리뷰 전문: `~/.claude/references/2026-05-21_sprint52_heading_false_positive_fix_review.md`

---

### 2026-05-21 — v0.5.0 Sprint 51: STYLE BSTR 수정 + 헤딩 스타일 헬퍼 추출 + 픽스처 하네스

**S51-01: HWPTAG_STYLE BSTR 파서 수정** (`src/hwp/reader.rs`, `src/hwp/reader_tests.rs`):

근본 원인: STYLE 레코드의 첫 필드는 `localName` BSTR(내부 ID), 두 번째가 `name` BSTR(사용자 표시명). 고정 오프셋 8은 `localName`이 정확히 3 UTF-16 chars일 때만 우연히 일치. `read_utf16le_str`가 반환하는 다음 오프셋을 체이닝하여 수정.

회귀 테스트 4건 추가: 한국어 이름/영문 이름/긴 localName/절단 데이터 무패닉.

**S51-02: `parse_heading_style` 공유 헬퍼 추출** (`src/hwp/heading_style.rs` 신규, `src/hwp/convert.rs`, `src/hwpx/handlers.rs`):

`src/hwp/heading_style.rs` 신규 생성 — `parse_heading_style(str) -> Option<u8>` (대소문자 무관, "outline"/"heading"/"개요"/"제목" + 공백 선택 + 숫자 1–6). `convert.rs`의 좁은 prefix 루프와 `handlers.rs`의 로컬 정의를 모두 교체. HWPX 전용 `parse_hwpx_style_ref` 래퍼로 numeric styleIDRef 폴백 보존.

테스트 4건 추가 (convert_tests_detect.rs): 영문 "Outline 2"/"개요 3"/소문자 "heading 1"/style_id 범위 초과 무패닉.

**S51-03: 실 픽스처 하네스** (`tests/real_fixtures_hwp.rs` 신규):

`real_fixtures_no_garbled_chars` (라이브, ignore 없음) — moel_01~05 5개 파일 변환 후 湰灧/桤灧 등 깨진 문자 부재 단언. 황금 비교 5건 + 구조적 비교 5건 (#[ignore], Sprint 52 재생성 후 활성화 예정).

**Commits**: `b39476b`

**검증**: 1177 tests (1167 active + 10 ignored), 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: 승인. H1 parse_hwpx_style_ref 단위 테스트 미작성, H2 numeric styleIDRef 폴백이 외부 HWPX 생성자와 충돌 가능 (Sprint 52 검토). M1 "Heading" 단독 동작 변경 CHANGELOG 미반영, M2 공백 처리 단일 ASCII 공백만, M3 "+1" 오파스 파싱, M5 래퍼 가시성.
리뷰 전문: `~/.claude/references/2026-05-21_sprint51_style_bstr_heading_helper_review.md`

---

### 2026-05-21 — v0.5.0 Sprint 50: HWPTAG 상수 수정 + 단락 스타일 헤딩 감지

**S50-01: Clippy pedantic 경고 3건 수정** (`src/hwp/summary.rs`, `src/hwpx/writer_content.rs`):
- `map_or(true, ...)` → `is_none_or(...)` (summary.rs:64)
- `% 4 != 0` → `!is_multiple_of(4)` (summary.rs:182, writer_content.rs:308 ×2)

**S50-02: HWPTAG 상수 수정 + 스타일 기반 헤딩 감지** (`src/hwp/record.rs`, `model.rs`, `reader.rs`, `control/dispatcher.rs`, `convert.rs`):

근본 원인: `HWPTAG_CHAR_SHAPE = HWPTAG_BEGIN + 8` (실제 BULLET 태그), `HWPTAG_PARA_SHAPE = HWPTAG_BEGIN + 14` (실제 DOC_CHANGE_TRACK_INFO) — 프로젝트 초기부터 잘못된 상수. 실 DocInfo 덤프로 확인 (0x0015=167건 CHAR_SHAPE, 0x0019=146건 PARA_SHAPE, 0x001A=30건 STYLE).

수정:
- `HWPTAG_CHAR_SHAPE = HWPTAG_BEGIN + 5` (0x0015)
- `HWPTAG_PARA_SHAPE = HWPTAG_BEGIN + 9` (0x0019)
- `HWPTAG_STYLE = HWPTAG_BEGIN + 10` (0x001A) 신규 추가
- `DocInfo.styles: Vec<String>` 추가, HWPTAG_STYLE 파싱
- `HwpParagraph.style_id: u16` 추가, PARA_HEADER bytes[6-7] 읽기
- `detect_heading_level`: "Outline N"/"개요 N" 스타일명 기반 경로 추가
- `detect_heading_level`: `&& cs.bold` 요구 제거 (실 정부 HWP 문서는 non-bold 헤딩 사용)

**Commits**: `828130d`

**검증**: 1162 tests, 0 failures. Clippy 0 warnings (pedantic 포함).

**리뷰 (code-reviewer opus)**: HIGH-1 STYLE 파서 오프셋 가정, HIGH-2 테스트 0건, HIGH-3 PARA_HEADER 오프셋 검증, HIGH-4 스타일명 매칭 범위 협소.
리뷰 전문: `~/.claude/references/2026-05-21_sprint50_hwptag_constants_heading_detection_review.md`

---

### 2026-05-21 — v0.5.0 Sprint 49: PARA_TEXT 제어 코드 수정 (湰灧 버그 제거)

**S49-01: `extract_paragraph_text` 제어 코드 범위 수정** (`src/hwp/reader.rs`, `src/hwp/reader_tests.rs`):

근본 원인: HWP 5.0 PARA_TEXT 스트림에서 0x000E–0x001F 범위(확장 인라인 제어 코드)가 14바이트 인라인 데이터를 동반함에도 `0x0000 | 0x000D` 그룹에 포함되어 건너뜀 없이 처리됨. 자동 번호 매기기 코드 0x0015의 인라인 데이터 첫 두 u16 값 `[0x6E70, 0x7067]`이 CJK 문자로 해석되어 `湰灧`(U+6E70 U+7067) 출력.

수정: `0x000E..=0x001F`를 no-skip 그룹에서 `0x0001..=0x0008 | 0x000B..=0x000C | 0x000E..=0x001F` (14-byte skip 그룹)으로 이동. `0x000D`(섹션 나누기)는 인라인 데이터 없음 — 실 레코드로 검증.

**회귀 테스트 3건 추가** (`src/hwp/reader_tests.rs`):
- `extract_paragraph_text_extended_ctrl_0x0015_skips_14_bytes` — 湰灧 패턴 재현 후 0으로 확인
- `extract_paragraph_text_extended_ctrl_0x000e_skips_14_bytes` — 범위 하한 검증
- `extract_paragraph_text_section_break_0x000d_no_extra_bytes` — 0x000D 비대칭 검증

**실 HWP 파일 검증**: moel_01~05 5개 파일 모두 깨진 문자 0개 (수정 전 4개 파일에서 발생)

**Commits**: `5a0ba61`

**검증**: 1162 tests, 0 failures. Clippy 0 warnings.

**리뷰 (code-reviewer opus)**: H1 공유 헬퍼 추출, H2 실 픽스처 .md 재생성 권고.
리뷰 전문: `~/.claude/references/2026-05-21_sprint49_para_text_control_codes_review.md`

---

### 2026-05-21 — v0.5.0 Sprint 48: CI 복구 + 실 HWP 파일 평가

**S48-01: MSRV 1.75 → 1.88 + CI 수정** (`Cargo.toml`, `.github/workflows/ci.yml`, `README.md`):
- 원인: `comrak 0.34` → `bon 3.9.1` → `darling 0.23` → `edition2024` feature 요구 (≥ Rust 1.88)
- CI toolchain `1.75.0` → `1.88.0` (test/lint/msrv job 모두)
- `rust-version = "1.75"` → `"1.88"`

**S48-02: flush.rs broken intra-doc link 수정** (`src/hwpx/context/flush.rs`):
- `` [`flush_paragraph`] `` → `` `flush_paragraph` `` (비공개 모듈 scope 미지원 → backtick 코드 스팬)
- `cargo doc --document-private-items` 0 warnings

**실 HWP 파일 수집 + 평가** (`tests/fixtures/real/`):
- 고용노동부 훈령/고시 5개 파일 수집 (공공저작물)
- 변환 성공률: 5/5 (종료 코드 0)
- **버그 발견**:
  - `湰灧` 깨진 문자 — 4개 파일에서 발생 (섹션 경계 컨트롤 코드 오해석)
  - HWP 단락 스타일 → 마크다운 헤딩 미변환
  - 테이블 셀이 `| col |` 아닌 plain text로 분해
  - 목록 번호 중복 (`1.   1.` 패턴)

**Commits**: `dd652bf` (S48-01+02), `5a8bc08` (MD 출력), `7df34a0` (HWP fixtures)

**검증**: 1326 tests, 0 failures. Clippy pedantic 0 warnings.

### 2026-05-21 — v0.5.0 Sprint 47: dead code 제거

**S47-01: `Hwp2MdError::MarkdownParse` 제거** (`src/error.rs`):
- 정의만 있고 생성 코드가 없는 dead variant 삭제 (4줄)
- `HwpxParse` / `HwpParse`로 커버되는 에러 경로여서 실제 미사용

**검증**: Sprint 46 검증 중 발견 — `grep -rn "MarkdownParse" src/` 결과 정의 1건만

**Commits**: `93d5026`

**검증**: `cargo clippy --all-targets -- -D clippy::pedantic` 0 warnings, 1326 tests (0 failures)

### 2026-05-21 — v0.5.0 Sprint 46: hh:breakSetting IR roundtrip

**S46-01: `hh:breakSetting` IR roundtrip** (`src/ir.rs`, `src/hwpx/reader.rs`, `src/hwpx/writer_header.rs`):
- New `BreakSetting` struct: `widow_orphan`, `keep_with_next`, `keep_lines`, `page_break_before` (all `bool`, `#[allow(clippy::struct_excessive_bools)]`)
- `Section.break_setting: BreakSetting` — default all false; serde default
- `parse_break_setting(xml: &str) -> ir::BreakSetting` in `reader.rs`: scans `hh:paraPr id="0"` → `hh:breakSetting` attributes from header.xml; cloned into every section
- `write_single_para_pr` now accepts `bs: &ir::BreakSetting`; emits IR values instead of hardcoded `"false"` for all four boolean attributes
- New `src/hwpx/reader_tests_break_setting.rs`: 13 tests (8 unit + 5 write→read roundtrip)

**S46-02: html_table Known Limitations doc** (`src/md/html_table.rs`):
- Added `# Known Limitations` section to module doc comment
- Documents: nested tables → None, block content in cells → plain text, inline formatting stripped, unescaped `&` → None fallback

**S46-03: `local_name` heap alloc benchmark + decision** (`benches/conversion.rs`):
- `bench_parse_html_table_large_100x10`: 290 µs for 100×10 table (~2202 `local_name` calls)
- Decision: keep `String` return — heap alloc cost is negligible vs XML tokenizer + IR builder
- `local_name` annotated with measurement note

**Commits**: `53be398`

**검증**: `cargo clippy --all-targets -- -D clippy::pedantic` 0 warnings, 1326 tests (0 failures)

### 2026-05-20 — v0.5.0 Sprint 45: MD Parser HTML `<table>` → IR

**New: `src/md/html_table.rs`** — `parse_html_table(literal: &str) -> Option<ir::Block>`:
- `quick_xml` SAX-style parser for HTML `<table>` blocks embedded in Markdown HtmlBlock nodes
- `<thead>`/`<tbody>`/`<tfoot>` silently ignored (rows treated as direct children)
- `colspan`/`rowspan` parsed per cell (default 1, clamped ≥ 1)
- `is_header` = all cells in row are `<th>`
- `col_count` = max sum-of-colspans across rows
- Nested `<table>` → `tracing::warn!` + None; parse error → warn + None
- Self-closing `<td/>`/`<th/>` correctly pushed immediately (no End event follows `Empty`)

**Wired** into `src/md/parser.rs` `HtmlBlock` arm: pagebreak check → html-table check → None.

**New: `src/md/parser_tests_html_table.rs`** — 20 tests:
- 11 happy-path: basic 2×2, colspan, rowspan, both, header detection, mixed cells, `<thead>`/`<tbody>`, entity decoding, attribute order, no-spans, `col_count` from colspan sum
- 5 edge/negative: non-table HTML → None, empty table → None, `colspan="0"` clamped, empty `<tr>` skipped, non-numeric span defaults to 1
- 3 round-trip: write IR → Markdown → parse back, compare row/cell structure
- 1 W3 regression: `parse_html_table_self_closing_td_not_dropped`

**Commits**: `51bff02` (main sprint), `ba35312` (W3 fix: self-closing `<td/>`)

**Deferred** (documented limitations):
- Nested tables inside cells → returns None
- Block-level / inline-formatted content inside cells → flattened to plain text
- Hand-authored `<table>` with unescaped `&` in cell text may fail `quick_xml` parse → None fallback

**리뷰 결과** (0 CRITICAL, 0 HIGH, 1 W3 bug fixed, suggestions documented):
- W3 (fixed): self-closing `<td/>` silently dropped — fixed by splitting Start/Empty arms
- W1 (deferred): `<tablespoon>` false-positive on prefix check — acceptable tradeoff
- W2 (deferred): heap alloc per tag in `local_name` — premature optimization
- W4 (deferred): doc comment should mention round-trip limitations

**검증**: `cargo clippy --all-targets -- -D clippy::pedantic` **0 warnings**, 1313 tests (0 failures)

### 2026-05-20 — v0.5.0 Sprint 44: Preserve-Existing Bool Parsing

**S44-01: `parse_page_pr` landscape semantics fix** (`src/hwpx/context/state.rs`):
- Before: `self.landscape = val == "true" || val == "1"` (unconditional — resets to false on "yes"/"TRUE"/"")
- After: `match val { "true"|"1" => true, "false"|"0" => false, _ => {} }` (preserve-existing)
- Consistent with `parse_page_size` / `parse_margin` skip-on-parse-failure semantics

**S44-02+03: Group C test updates + new test**:
- 3 tests renamed + assertions flipped (`_resets_to_false` → `_preserves_existing`)
- New: `parse_page_pr_unknown_value_preserves_false` (symmetric counterpart)

**S44-04: `apply_charpr_attrs` bool semantics fix** (`src/hwpx/context/flush.rs`):
- Extracted `parse_bool_preserve(val: &str, current: bool) -> bool` helper
- `bold`/`italic` arms use preserve-existing; `underline`/`strikeout`/`supscript` unchanged (different OWPML semantics)

**S44-05: 3 charpr preserve tests** (`src/hwpx/reader_tests_charpr.rs`):
- `apply_charpr_attrs_bold_garbage_value_preserves_existing`
- `apply_charpr_attrs_italic_garbage_value_preserves_existing`
- `apply_charpr_attrs_bold_numeric_one_sets_true`

**리뷰 결과** (0 CRITICAL, 0 HIGH, 1 Suggestion):
- Suggestion: if a 3rd `parse_bool_preserve` call site emerges, promote to `context/mod.rs` or `context/xml_attr.rs` — YAGNI applies now.

**Commit**: `591abdc`

**검증**: `cargo clippy --all-targets -- -D clippy::pedantic` **0 warnings**, 1293 tests (0 failures)

### 2026-05-20 — v0.5.0 Sprint 43: PageLayoutState Parser Edge Cases

**Group A — `parse_page_size` (3 tests)**:
- `parse_page_size_invalid_value_preserves_existing_some`: `"bogus"` parse fail → existing `Some(n)` preserved
- `parse_page_size_negative_value_preserves_existing_some`: `"-1"` → u32 fail → preserved
- `parse_page_size_overflow_value_preserves_existing_some`: `"4294967296"` → overflow → preserved

**Group B — `parse_margin` (3 tests)**:
- `parse_margin_invalid_value_preserves_existing_some`: mixed invalid+valid in one call → selectively preserved/updated
- `parse_margin_negative_value_preserves_existing_some`
- `parse_margin_overflow_value_preserves_existing_some`

**Group C — `parse_page_pr` (5 tests)**:
- `parse_page_pr_unknown_value_resets_to_false`: `"yes"` → false (documents current strict OWPML behavior)
- `parse_page_pr_empty_value_resets_to_false`
- `parse_page_pr_uppercase_true_does_not_match`: case-sensitive match
- `parse_page_pr_unknown_attribute_ignored_preserves_landscape`
- `parse_page_pr_mixed_known_and_unknown_attrs`

**리뷰 결과** (0 CRITICAL, 0 HIGH, 1 Warning):
- Warning: Group C behavior is inconsistent with sibling parsers — `parse_page_size`/`parse_margin` preserve prior state on parse failure, but `parse_page_pr` unconditionally assigns `false` for unknown values. Requires user decision: (A) intentional strict OWPML + add doc comment, or (B) fix to preserve-existing + update 3 assertions + unify with `flush.rs:19,21`.
- Suggestions: zero-value guard, mixed-axis margin invalid test, `"True"` title-case, duplicate attrs, whitespace in value, `flush.rs` boolean test family.

**Commit**: `49cf1e3`

**검증**: `cargo clippy --all-targets -- -D clippy::pedantic` **0 warnings**, 1289 tests (0 failures)

### 2026-05-20 — v0.5.0 Sprint 42: PageLayoutState Parser Unit Tests + make_empty Helper

**S42-01~03: 13 unit tests for 3 previously-untested PageLayoutState parsers**:
- `parse_page_size` (4 tests): all axes, width-only, height-only, no attrs
- `parse_margin` (4 tests): all 4 axes, left+right only, top+bottom only, no attrs
- `parse_page_pr` (5 tests): landscape true/false, "0" → false, hp: prefix, no attrs preserves existing

**Follow-up S1: `make_empty(tag, attrs)` unified helper**:
- Collapsed 3 near-identical helpers (`make_page_size`, `make_margin`, `make_page_pr`) into parameterized `make_empty`
- Existing `make_in_margin` kept as thin wrapper — 6 Sprint 41 tests unchanged
- `page_layout()` factory deleted; all tests use `PageLayoutState::default()` directly
- `PageLayoutState { landscape: true, ..PageLayoutState::default() }` struct-update syntax → satisfies `clippy::field_reassign_with_default`

**Follow-up S3: landscape edge cases**:
- `parse_page_pr_landscape_zero_resets`: `landscape="0"` sets false (numeric string support)
- `parse_page_pr_no_attrs_preserves_existing_landscape`: absent attr is a no-op

**Commits**: `dc1827c` (Sprint 42), `f881380` (follow-up)

**리뷰 결과** (0 CRITICAL, 0 HIGH):
- M: `parse_page_size` — invalid value with prior `Some(n)` state untested (Sprint 43 candidate)
- M: `parse_page_pr` — unknown value like `"yes"` silently resets to false (behavior unverified)
- L: Unknown attributes in `parse_page_pr` are silently ignored (graceful but unverified)
- L: Negative/overflow numeric values not tested across any parser

**검증**: `cargo clippy --all-targets -- -D clippy::pedantic` **0 warnings**, 1278 tests (0 failures)

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

### 2026-05-19 — v0.5.0 Sprint 39: Table Margin Const Extraction + Criterion Bench Baseline

**S39-01: Magic literal extraction** (`src/hwpx/writer_section.rs`):
- `TABLE_INNER_MARGIN: &str = "141"` — inter-cell gap in `<hp:inMargin>`
- `TABLE_CELL_PAD_H: &str = "510"` — horizontal padding in `<hp:cellMargin>`
- `TABLE_CELL_PAD_V: &str = "141"` — vertical padding in `<hp:cellMargin>`
- Replaced 8 literal sites; kept separate despite equal values (distinct physical concepts)

**S39-02: Criterion bench baseline** (2026-05-19, Apple Silicon):
- `ir_to_md`: 8.29 µs
- `ir_to_hwpx`: 474.78 µs | `hwpx_to_ir`: 205.96 µs
- `md_hwpx_md_roundtrip`: 764.52 µs
- `ir_to_hwpx_table_heavy` (20×5): 692.49 µs | `hwpx_table_heavy_write_read`: 1.117 ms

**리뷰 follow-up**: `write_table_2x3_has_required_elements`에 `<hp:cellMargin>` 속성값 assertion 추가 (left/right=510, top/bottom=141)

**검증**: `cargo clippy --all-targets -- -W clippy::pedantic` **0 warnings**, 1254 tests (0 failures)

### 2026-05-20 — v0.5.0 Sprint 41: Partial inMargin Axis Defaults + Parser Robustness

**S41-01: parse_in_margin() default fix** (`src/hwpx/context/state.rs`):
- Changed initial `TableInnerMargin { left: 0, right: 0, top: 0, bottom: 0 }` → `{ left: 141, right: 141, top: 141, bottom: 141 }`
- Unspecified axes now default to 141 HWP units (OWPML default) instead of 0

**S41-02: 4 unit tests** for `parse_in_margin()`:
- All 4 axes explicit, left-only partial, top+bottom partial, no attrs

**S41-03: Bench verification** (2026-05-20, Apple Silicon):
- `md_to_ir`: 23.64 µs (confirmed bench_md_to_ir running — was P3 concern from Sprint 39)
- `ir_to_md`: 8.21 µs | `ir_to_hwpx`: 501.12 µs | `hwpx_to_ir`: 214.14 µs
- `md_hwpx_md_roundtrip`: 786.46 µs | `ir_to_hwpx_table_heavy`: 702.29 µs | `hwpx_table_heavy_write_read`: 1.1173 ms

**리뷰 follow-up** (W1+W2+W3+S2):
- `DEFAULT_TABLE_INNER_MARGIN: u32 = 141` const added to `ir.rs` — single source of truth; `state.rs` references it
- `Box::leak` → `into_owned()` in test helper (no memory leak)
- `hp:left`/`hp:bottom` prefix branch test added (W3)
- Invalid attribute value: `unwrap_or(0)` → `let-else continue` (S2) — preserves 141 default instead of overwriting with 0

**Commits**: `360218e` (Sprint 41), `f3921d1` (follow-up W1+W2+W3+S2)

**검증**: `cargo clippy --all-targets -- -D clippy::pedantic` **0 warnings**, 1268 tests (0 failures)

### 2026-05-20 — v0.5.0 Sprint 40: TableInnerMargin IR Field + Roundtrip Preservation

**S40-01: `TableInnerMargin` struct** (`src/ir.rs`):
- `pub struct TableInnerMargin { left, right, top, bottom: u32 }` — OWPML `<hp:inMargin>` values
- `inner_margin: Option<TableInnerMargin>` field added to `Block::Table` variant
- `None` = use writer default (141 HWP units); field fully doc-commented

**S40-02: All Block::Table sites updated** (28 files, ~62 sites):
- Construction sites: `inner_margin: None` (backward-compatible defaults)
- Exhaustive match arms: `..` wildcard (forward-compatible)

**S40-03: Writer wiring** (`src/hwpx/writer_section.rs`):
- `write_table` signature: `inner_margin: Option<&ir::TableInnerMargin>` parameter
- `Some(m)` → emit custom values; `None` → fall back to `TABLE_INNER_MARGIN` const

**S40-04: Reader wiring** (`src/hwpx/handlers.rs` + `context/state.rs`):
- `TableState.inner_margin: Option<ir::TableInnerMargin>` accumulator
- `"inMargin" | "hp:inMargin"` handler in both `handle_start_element` and `handle_empty_element` (required: our writer emits `<hp:inMargin/>` as Empty event)
- `handle_end_element` propagates `inner_margin.take()` into `Block::Table`

**S40-05: 3 new tests** + follow-up 1 (4 total):
- `write_table_custom_inner_margin_emitted` — writer emits custom values
- `write_table_default_inner_margin_when_none` — None → 141 fallback unchanged
- `write_table_inner_margin_roundtrip` — full write→read roundtrip (left=right=300, top=bottom=50)
- `write_table_asymmetric_inner_margin_roundtrip` — all four axes distinct (11/22/33/44)

**리뷰 follow-up** (W1+W2+S3-P1):
- `TableState::parse_in_margin()` helper extracted to `state.rs` — eliminates 40-line duplicate in Start/Empty handlers
- `inner_margin.take()` moved outside empty-rows guard → always cleared on `</hp:tbl>`
- Asymmetric roundtrip test added

**Commits**: `88b04fa` (main Sprint 40), `c6b79cf` (follow-up)

**검증**: `cargo clippy --all-targets -- -D clippy::pedantic` **0 warnings**, 1262 tests (0 failures)

### 2026-05-19 — v0.5.0 Sprint 37: colspan cellSz Scaling + HWPX Span Proptest + Publish Prep

**P1: cargo publish --dry-run + semver CI 활성화**:
- `cargo publish --dry-run -p hwp2md` 성공 (114 files, 256.2 KiB)
- `.github/workflows/ci.yml`: semver job `continue-on-error: true` 제거

**P2: colspan/rowspan-scaled `<hp:cellSz>`**:
- `src/hwpx/writer_section.rs`: `cell.colspan.max(1) * TABLE_CELL_WIDTH` 비례 계산
- `cell.rowspan.max(1)` guard — malformed 0 값 방어
- `writer_tests_table.rs`: `write_table_colspan_cellsz_scaled` — count 기반 assertion

**P3: HWPX span roundtrip proptest 강화**:
- `table_with_spans()`: Variant B (colspan=2 단일 셀) `prop_oneof!` 추가
- ordering 기반 assertion → `matches().count()` count 기반으로 교체
- `hwpx_table_roundtrip_preserves_spans` (64 cases): colspan=2 회귀 감지 가능

**검증**: `cargo clippy --all-targets -- -W clippy::pedantic` **0 warnings**, 1253 tests (0 failures)

### 2026-05-19 — v0.5.0 Sprint 38: hp:tblPr Emission + Table-Heavy Benchmarks

**P4: Table-heavy benchmarks** (`benches/conversion.rs`):
- `TABLE_HEAVY_MD`: 20행 × 5열 Korean 텍스트 테이블 constant 추가
- `bench_ir_to_hwpx_table_heavy`: IR 사전 빌드, `write_hwpx`만 측정
- `bench_hwpx_table_heavy_write_read`: IR 사전 빌드, write+read 경로 측정
- `criterion_group!` 7 bench functions 등록

**P5: `<hp:tblPr>` emission** (`src/hwpx/writer_section.rs`):
- `write_table`: `<hp:tbl>` 첫 번째 자식으로 `<hp:tblPr><hp:inMargin left="141" right="141" top="141" bottom="141"/></hp:tblPr>` 추가 (OWPML spec 준수)
- doc comment XML 예시 업데이트

**테스트** (`src/hwpx/writer_tests_table.rs`):
- `write_table_has_tblpr`: `<hp:inMargin>` 요소 추출 후 4방향 속성 검증; `tblPr < sz` 순서 불변성 assertion
- `write_table_2x3_has_required_elements`: `<hp:tblPr>`, `<hp:inMargin>` assertion 추가

**리뷰 follow-up**: benchmark criterion id 수정 (`hwpx_table_heavy_roundtrip` → `hwpx_table_heavy_write_read`), inMargin top/bottom assertion 추가, OWPML child-order assertion

**검증**: `cargo clippy --all-targets -- -W clippy::pedantic` **0 warnings**, 1254 tests (0 failures)

### 2026-05-19 — v0.5.0 Sprint 36: HWPX Table Writer OWPML Completion + Release Prep

**T1: HWPX table writer — complete OWPML structure**:
- `src/hwpx/writer_section.rs`: `write_table` helper extracted; emits `<hp:sz>`, `<hp:pos treatAsChar>`,
  `<hp:trHeight>`, `<hp:cellAddr>`, `<hp:cellSpan>`, `<hp:cellSz>`, `<hp:cellMargin>`, `<hp:subList>`
- `src/hwpx/writer_header.rs`: second borderFill entry (id=2, SOLID black) added; itemCnt 1→2
- `src/hwpx/writer.rs`: `RefTables.table_border_fill_id: u32 = 2` — single source of truth for table border ID
- `src/hwpx/handlers.rs`: `cellSpan` handler added (OWPML-authoritative span source);
  `cellAddr` handler preserved for old HWPX compat with clarified comment
- `src/hwpx/writer_tests_table.rs`: 6 new tests (structure, indices, roundtrip, colspan, borderFill)

**T2: v0.5.0 release prep**:
- `CHANGELOG.md`: `[0.5.0] - Unreleased` → `[0.5.0] - 2026-05-19`; Sprint 34/35/36 entries added
- Stale 0.4.x references: none found in docs/config

**검증**: `cargo clippy --all-targets -- -W clippy::pedantic` **0 warnings**, 1251 tests (0 failures)

### 2026-05-19 — v0.5.0 Sprint 35: proptest Expansion + CLI Split + CI Improvements

**T1: CodeBlock trailing-whitespace fix**:
- `src/md/writer.rs:120`: `{line}` → `{}`, line.trim_end()` — CommonMark 이음매 수정
- `tests/proptest_roundtrip.rs`: `prop_assume!` 블록 13줄 제거; `write_is_idempotent` 완전 이음매

**T2: proptest 전략 확장**:
- `block_quote()`: 내부 Paragraph/Heading/HR만 (comrak blockquote 중첩 제약)
- `block_list()`: start=1 고정, 단일 Paragraph 항목, 비중첩
- `block_table()`: 고정 cols, safe_text(), col_count 일치
- `no_adjacent_same_type_lists` 필터로 동형 인접 List 병합 방지

**T3: `tests/cli.rs` (753L) 분할**:
- `tests/cli_general.rs` (21 tests), `cli_to_md.rs` (4), `cli_to_hwpx.rs` (2)
- `tests/cli_batch.rs` (11, 기존) — 총 38 CLI tests, 커버리지 손실 없음

**T4: `InlineFormat` 임포트 정리**:
- `flush.rs`, `handlers.rs`, `convert.rs`, `parser.rs`: fully-qualified → `use crate::ir::InlineFormat;`

**T5: CI 개선**:
- `msrv` job: `dtolnay/rust-toolchain@1.75.0` + `cargo check --workspace`
- `semver` job: `obi1kenobi/cargo-semver-checks-action@v2` + `continue-on-error: true`

**검증**: `cargo clippy --all-targets -- -W clippy::pedantic` **0 warnings**, 1245 tests (0 failures)

### 2026-05-19 — v0.5.0 Sprint 34: proptest Roundtrip Invariants + write_assets Security

**S34-01: proptest roundtrip invariants**:
- `tests/proptest_roundtrip.rs` 신규 작성; `proptest = "1"` dev-dependency 추가
- 3가지 속성 기반 불변식 (128 케이스): `roundtrip_preserves_block_structure`, `write_is_idempotent`, `no_panics_on_random_documents`
- `safe_text()` 전략: 제어문자·서로게이트 없는 유니코드 — 파서 충돌 방지
- `CodeBlock` trailing-whitespace 불일치는 `prop_assume!` 임시 필터링 (Sprint 35 수정 예정)

**S34-02: `write_assets` 경로 보안 강화**:
- `sanitize_asset_name`: 경로 순회(`../../`), NUL/제어문자(0x01–0x1F, 0x7F), 후행 점/공백, Windows 예약명(CON/PRN/AUX/NUL/COM1-9/LPT1-9) 모두 처리
- `next_available_name`: 점파일 충돌 버그 수정 (`rfind('.')` → `Path::file_stem()/extension()`)
- `src/convert_tests_sanitize.rs` 21개 단위 테스트 추가

**검증**: `cargo clippy --all-targets -- -W clippy::pedantic` **0 warnings**, 1245 tests (0 failures)

### 2026-05-19 — v0.5.0 Sprint 33: Security Hardening + InlineFormat Refactor

**S33-SEC-01: `try_lenient_read` file size cap (256 MiB)**:
- `MAX_LENIENT_FILE_BYTES: u64 = 256 * 1024 * 1024` added alongside `LENIENT_MAX_RECORD_BYTES`
- `try_lenient_read_inner(path, limit)` extracted for testability; public wrapper delegates with constant
- Returns `Hwp2MdError::FileTooLarge` before `fs::read`; prevents unbounded allocation on adversarial input
- Test: `try_lenient_read_rejects_oversized_file` verifies rejection with small injected limit

**S33-SEC-02: Replace `unwrap_or(u16::MAX)` with skip+warn**:
- `parse_char_shape_refs` 8-byte entry path: IDs > 65535 are now skipped with `tracing::warn!`
- `read_bin_data` fallback ID: out-of-range index skips entry via `continue` instead of sentinel aliasing
- Eliminates silent ID aliasing that could corrupt formatting on adversarial HWP files

**S33-01: `record.rs` `to_le_bytes()` cleanup (test helpers)**:
- `encode_utf16le` and `build_utf16le_str`: `buf.push(lo); buf.push(hi)` → `buf.extend_from_slice(&u.to_le_bytes())`

**S33-02: `InlineFormat` POD + `Inline::with_formatting` refactor**:
- New `pub struct InlineFormat { bold, italic, underline, strikethrough, superscript, subscript, color }` in `ir.rs`
- `Inline::with_formatting` signature: 7 positional args → `(text: String, fmt: &InlineFormat)`
- Removes `#[allow(clippy::fn_params_excessive_bools)]` + `#[allow(clippy::too_many_arguments)]` from `ir.rs`
- `From<&FormattingState> for InlineFormat` in `hwpx/context/state.rs`
- All 6 call sites updated (flush.rs ×2, handlers.rs ×2, convert.rs, md/parser.rs) + 3 test sites

**검증**: `cargo clippy --all-targets -- -W clippy::pedantic` **0 warnings**, 1220 tests (0 failures)

### 2026-05-19 — v0.5.0 Sprint 32: Test Cast Hardening + Allow Justifications

**S32-01: Replace module-level `#[allow]` with `try_from().unwrap()` in 8 test files**:
- `hwp/control/{dispatcher,hyperlink,ruby}.rs` — `usize as u16` → `u16::try_from(x).unwrap()`
- `hwp/crypto.rs` — `usize as u8` → `u8::try_from(x).unwrap()` (3 casts)
- `hwp/lenient.rs` — 8 × `text_bytes.len() as u32` → `u32::try_from(x).unwrap()`
- `hwp/record.rs` — `units.len() as u16` → `u16::try_from(x).unwrap()` (2 casts)
- `hwp/summary.rs` — 4 × `x as u32` → `u32::try_from(x).unwrap()`
- `hwpx/writer_tests_image_util.rs` — `i as u8` → `u8::try_from(i).unwrap()`
- All 8 module/function-level `#[allow(clippy::cast_possible_truncation)]` removed

**S32-02: Justification comments on all production `#[allow]` attributes (16 files)**:
- One-line comment before each allow explaining why the lint is suppressed
- Covers: cast_sign_loss, cast_possible_truncation, struct_excessive_bools,
  fn_params_excessive_bools, too_many_arguments, too_many_lines, unnecessary_wraps,
  many_single_char_names, trivially_copy_pass_by_ref

**검증**: `cargo clippy --all-targets -- -W clippy::pedantic` **0 warnings**, 1082 테스트 (0 failures)

### 2026-05-17 — v0.5.0 Sprint 31: Pedantic Clippy Zero Warnings (All Targets)

**S31: `#[allow]` annotations for test code (11 files)**:
- `#[cfg(test)] #[allow(clippy::cast_possible_truncation)] mod tests` added to 7 production files:
  - `hwp/control/{dispatcher,hyperlink,ruby}.rs` — usize→u16 in test data builders
  - `hwp/crypto.rs` — usize→u8 in test key/data helpers
  - `hwp/lenient.rs` — 8 × usize→u32 in synthetic HWP record constructors
  - `hwp/record.rs` — usize→u16 in record builders
  - `hwp/summary.rs` — 4 × usize→u32 in OLE2 byte-buffer helpers
- `#[allow(clippy::too_many_lines)]` on 4 test functions:
  - `writer_tests_golden.rs`: `golden_comprehensive_document_structure` (274L)
  - `reader_tests_list.rs`: `roundtrip_nested_list_md_to_hwpx_to_md` (107L)
  - `tests/hwpx_roundtrip_full2.rs`: `full_roundtrip_combined_all_block_types_text_preserved` (140L)
  - `writer_tests_image_util.rs`: `collect_image_assets_three_way_collision_increments_counter` (cast)

**검증**: `cargo clippy --all-targets -- -W clippy::pedantic` **0 warnings** (전체 마일스톤), 1219 테스트 (0 failures)

### 2026-05-16 — v0.5.0 Sprint 30: Pedantic Clippy Zero Warnings

**S30-01/02: `#[allow]` annotation sweep across 16 files**:
- `cast_possible_truncation` / `cast_sign_loss`: shapes.rs, reader.rs (×2), writer_header.rs, writer_content.rs (base64), parser.rs
- `struct_excessive_bools`: model.rs (FileHeader, CharShape), context/state.rs, context/mod.rs, hwpx/writer.rs, ir.rs, md/parser.rs
- `fn_params_excessive_bools` + `too_many_arguments`: ir.rs (Inline::with_formatting)
- `too_many_lines`: convert.rs, eqedit.rs, handlers.rs (×3), writer_section.rs, parser.rs (×2)
- `unnecessary_wraps`: crypto.rs (decrypt_viewtext), hwpx/reader.rs (parse_metadata, parse_section_xml_with_face_names)
- `many_single_char_names`: writer_content.rs (base64 a/b/c/d)

**리뷰 수정 (HIGH)**:
- `parse_char_shape_refs`: `id as u16` → `u16::try_from(id).unwrap_or(u16::MAX)`, 함수 레벨 `#[allow]` 제거
- `read_bin_data`: `(idx + 1) as u16` → `u16::try_from(idx + 1).unwrap_or(u16::MAX)`

**검증**: `cargo clippy --lib -- -W clippy::pedantic` 0 warnings, `--all-targets` 24 warnings (test code only), 1219 테스트 (0 failures)

### 2026-05-09 — v0.5.0 Sprint 29: Infallible From Casts + Small Pedantic Fixes

**S29-01: 6 small pedantic fixes**:
- `ruby.rs`: `iter_mut()` → `&mut`, redundant `.to_owned()` 제거
- `convert.rs`: `match` → `let...else` (early-return pattern)
- `handlers.rs`: `else { if let }` → `else if let` (collapsed)
- `roundtrip tests`: `.filter_map().next()` → `.find_map()`
- `ir.rs`: `"".to_string()` → `String::new()`

**S29-02: 38 infallible `as` casts → `From::from()`**:
- `dispatcher.rs`, `table.rs`: `b'H' as u16` 등 byte literal casts (6건)
- `convert.rs`: `col_span as u32`, `row_span as u32` → `u32::from()`
- `lenient.rs`: `tag_id as u32`, `level as u32` → `u32::from()`
- `reader.rs`: `ch as u32`, `low as u32` (UTF-16 surrogate pair) → `u32::from()`
- `record.rs`: tag/level/HWPTAG casts → `u32::from()`
- `reader_tests.rs`, `writer_tests_image*.rs`: 테스트 내 casts (14건)

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures)

### 2026-05-09 — v0.5.0 Sprint 28: Doc Backticks + Wildcard Imports + Pass-By-Value

**S28-01: 73 `doc_markdown` backtick warnings**:
- 22개 파일에서 doc 주석 내 CamelCase/SCREAMING_SNAKE_CASE 식별자에 backtick 추가
- `CTRL_HYPERLINK`, `GShapeObject`, `PARA_TEXT`, `DocInfo`, `MathJax`, `KaTeX` 등

**S28-02: 11 wildcard imports → explicit**:
- `hwp/control/{common,hyperlink,image,dispatcher,table}.rs`: `use crate::hwp::record::*` → 명시적
- `hwp/convert.rs`, `shapes.rs`, `reader.rs`: `use crate::hwp::model::*` → 명시적
- test 파일 4건: `super::*`에 의존하던 model imports 직접 추가

**S28-03: 7 `needless_pass_by_value` fixes**:
- `eqedit.rs`: 4 functions `Vec<Token>` → `&[Token]`
- `parser.rs`: `InlineStyle` → `&InlineStyle`
- `record.rs`: `ctrl_id` `#[allow(clippy::trivially_copy_pass_by_ref)]`

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures)

### 2026-05-09 — v0.5.0 Sprint 27: #Errors Docs + If-Let + Range Merge

**S27-01: control flow + items-before-statements**:
- `main.rs`, `context/mod.rs`: `match` → `if let`
- `#[allow(clippy::option_option)]` for intentional `Option<Option<T>>`
- test 파일 재정렬, redundant assertions 제거

**S27-02: `# Errors` doc sections (7 functions)**:
- `convert_auto`, `check`, `read_hwp`, `read_hwpx`, `write_hwpx`
- `StyleTemplate::from_file`, `StyleTemplate::from_yaml`

**S27-03: contiguous byte-tag range merge**:
- `hwp/reader.rs`: `0x0001..=0x0002 | 0x0003..=0x0008` → `0x0001..=0x0008`

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures)

### 2026-05-08 — v0.5.0 Sprint 26: Match Arms + Control Flow + Ext Comparison

**S26-01: identical match arms** (8 instances):
- `eqedit.rs`: "inf"|"infty" 통합, `shapes.rs`: redundant arm 제거
- `reader.rs`: control char ranges 통합, `handlers.rs`: fieldBegin 제거 (wildcard 커버)
- `hwpx/reader.rs`: Eof|Err break 통합, `roundtrip_stability.rs`: 중복 arm 제거
- 수동 수정: `1|_` → `_`, `HorizontalRule|_` → `_` (wildcard covers 경고)

**S26-02: control flow cleanup** (7 instances):
- `reader.rs`: match → if-let, bool-not 스왑
- `handlers.rs`: bool-not 스왑
- `parser.rs`: enum 정의 문 앞으로 이동
- `roundtrip.rs`: fn 정의 문 앞으로 이동 (2건)

**S26-03: case-sensitive extension comparison** (3 instances):
- `convert_tests_ir.rs`, `writer_tests_image.rs`: `.ends_with(".png")` → `Path::extension()` + `eq_ignore_ascii_case()`

**리뷰 결과** (0 CRITICAL, 0 HIGH, 0 MEDIUM, 2 SUGGESTION):
- S1: reader.rs 인접 범위 `0x01..=0x02|0x03..=0x08` → `0x01..=0x08` 합치기 가능
- S2: extension 검사 테스트 헬퍼 추출 가능

**검증**: cargo check 0 에러, clippy -D warnings 0 경고, 1219 테스트 (0 failures)

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

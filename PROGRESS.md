# hwp2md — Progress

## 현재 상태: Phase 5 완료 (Parser 고도화 + 테스트 커버리지)

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

### 진행 중

없음

### 미착수
- [ ] Phase 5: HWPX 라이터 고도화 (스타일, 이미지, 템플릿)
- [ ] Phase 6: CLI 완성 + 배포

## 중기 개선 로드맵 (Phase 1.5)

### 1순위: 테스트 기반 구축 (M1) ✅ 완료
- [x] 단위 테스트: `extract_paragraph_text`, `parse_char_shape`, 서로게이트 페어 처리
- [x] 통합 테스트: IR→MD→IR, MD→IR→MD 라운드트립 (14건)
- [x] 103 테스트 전체 통과
- [ ] 커버리지 80%+ 측정 (tarpaulin)
- [ ] 샘플 HWP/HWPX 파일 기반 통합 테스트

### 2순위: 남은 HIGH 이슈 ✅ 완료
- [x] H2: CharShape 속성 오프셋 46-49 수정 (HWP 5.0 스펙 대조)
- [x] H3: 제목 감지 임계값 상수화 + 단위 문서화
- [x] H5: charPr 위치 확인 (이미 정확) + 문서화
- [x] H8: frontmatter keywords YAML 파싱

### 3순위: MEDIUM 이슈
- [x] M3: HWPX writer Result 전파 (`?` 사용)
- [x] M6: YAML escape \n/\r/\t 추가
- [x] M8: `parse_heading_style` 끝 숫자 추출
- [x] M10: encoding_rs, serde_yaml 제거
- [x] M7: HWP OLE2 SummaryInformation 메타데이터 추출 ✅ (14694df)
- [x] M9: BIN 스트림 doc_info.bin_data_entries 기반 최적화 ✅ (14694df)
- [x] H4: `--style` 파라미터 tracing::warn 경고 ✅ (14694df)

## 장기 개선 로드맵 (Phase 2~6)

### 아키텍처
- [x] reader.rs 서브모듈 분할 (2057→828+781+434+238) ✅
- [x] hwpx/reader.rs 분할 (1895→931+968) + ParseContext dispatch ✅ (35ea51f)
- [ ] ParseContext 19필드 → 타입 상태 패턴 또는 빌더 분리
- [ ] Reader/Writer trait 정의 (HWP/HWPX/MD 공통 인터페이스)
- [x] HwpDocument → IR 변환에서 제어 문자 파싱 (테이블/이미지/각주) ✅

### HWP 파서 (Phase 2+2b) — 완료 항목
- [x] CTRL_TABLE + LIST_HEADER 레코드로 테이블 구조 파싱 ✅
- [x] BinData 참조 해결 → 이미지 인라인 삽입 ✅
- [x] CTRL_FOOTNOTE/ENDNOTE 파싱 ✅
- [x] 하이퍼링크 URL 추출 (CTRL_HYPERLINK + parse_hyperlink_url) ✅
- [x] DRM/암호화 감지 (has_drm 비트 + Hwp2MdError) ✅
- [x] parse_summary_bytes 분리 + OLE2 단위 테스트 ✅

### HWP 파서 — 남은 항목
- [x] EQEDIT 스크립트 → LaTeX 변환 고도화 ✅ (da92270)
- [ ] 샘플 HWP/HWPX 파일 기반 통합 테스트
- [ ] 커버리지 80%+ 측정 (tarpaulin)

### 배포 (Phase 6)
- [x] GitHub Actions CI (build + test + clippy) ✅ (37d2645)
- [ ] crates.io 배포 준비
- [ ] 배치 변환 CLI 옵션

### HWPX 파서 — Phase 3 완료
- [x] charPr 중복 제거 (apply_charpr_attrs 헬퍼) ✅
- [x] `<li>` 핸들러 + list_item 컨텍스트 ✅
- [x] 각주/미주 파싱 (fn/en/footnote/endnote) ✅
- [x] 각주 참조 (noteRef, ctrl) ✅
- [x] BinData 참조 해결 (build_bin_map + resolve_bin_refs) ✅
- [x] 교차 컨텍스트 라우팅 수정 (lineBreak/img/noteRef) ✅
- [x] ParseContext 디스패치 메서드 통합 ✅ (35ea51f)
- [ ] 샘플 HWPX 파일 기반 통합 테스트

### HWPX 테스트 — Phase 2d
- [x] parse_section_xml 단위 테스트 42건 ✅ (5e297d7)
- [x] Dead code clippy 경고 0건 (CI 호환) ✅
- [ ] 샘플 HWPX 파일 기반 통합 테스트

## 변경 이력

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

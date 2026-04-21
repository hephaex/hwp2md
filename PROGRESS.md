# hwp2md — Progress

## 현재 상태: Phase 2 완료 (HWP 제어 문자 파싱 + 메타데이터)

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

### 진행 중

없음

### 미착수

- [ ] Phase 2b: HWP 파싱 고도화 (하이퍼링크 URL 추출, EQEDIT→LaTeX 고도화, DRM 감지)
- [ ] Phase 3: HWPX 파싱 고도화 (스타일, colspan, BinData)
- [ ] Phase 4: Markdown 렌더러 고도화 (GFM 검증, 이미지 옵션)
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
- [ ] reader.rs 분할 (현재 2000+ lines → table/image/summary 서브모듈)
- [ ] ParseContext 19필드 → 타입 상태 패턴 또는 빌더 분리
- [ ] Reader/Writer trait 정의 (HWP/HWPX/MD 공통 인터페이스)
- [x] HwpDocument → IR 변환에서 제어 문자 파싱 (테이블/이미지/각주) ✅

### HWP 파서 (Phase 2) — 완료 항목
- [x] CTRL_TABLE + LIST_HEADER 레코드로 테이블 구조 파싱 ✅
- [x] BinData 참조 해결 → 이미지 인라인 삽입 ✅
- [x] CTRL_FOOTNOTE/ENDNOTE 파싱 ✅

### HWP 파서 (Phase 2b) — 남은 항목
- [ ] 하이퍼링크 URL 추출 (CTRL_HEADER에서 URL 바이트 파싱)
- [ ] EQEDIT 스크립트 → LaTeX 변환 고도화
- [ ] DRM/암호화 감지 메시지 개선
- [ ] read_summary_info 단위 테스트 (OLE2 바이트 버퍼 기반)

### 배포 (Phase 6)
- [ ] GitHub Actions CI (build + test + clippy)
- [ ] crates.io 배포 준비
- [ ] 배치 변환 CLI 옵션

## 변경 이력

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

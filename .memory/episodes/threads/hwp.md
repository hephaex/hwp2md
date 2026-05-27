# Thread: hwp2md

HWP/HWPX ↔ Markdown 변환기 (Rust). v0.5.0 published to crates.io.

## Timeline

### 2026-05-22 — Sprint 67-69

- Sprint 67 docs: tab terminator + U+202F narrow NBSP behavioral. 1234 tests.
- Sprint 68: Unicode White_Space coverage 완성 (U+FEFF, U+205F). 1234 tests.
- Sprint 69: 편(Part)→H1 감지 구현. heading_style.rs 추출. HWPX tier-4 연동. 1240+31 tests.
- Build fix: pub(crate) use 패턴으로 is_heading_terminator 가시성 해결.

### 2026-05-22 — Sprint 60-66 (이전 세션)

- Sprint 60-65: is_heading_terminator terminator matrix 완성 (bracket/punctuation behavioral).
- Sprint 66: U+3000/U+00A0/U+200B + is_heading_terminator doc comment 수정.
- Sprint 67: tab terminator path + U+202F behavioral.

### 2026-05-21 — Sprint 54-59 (이전 세션)

- Sprint 54: detect_korean_regulation_heading 구현 (장/절/조 tier-4).
- Sprint 55: trim_start 복원 + 100자 상한.
- Sprint 56: suffix 경계 검사 + PARA_HEADER nStyleID 수정.
- Sprint 57-59: is_heading_terminator 정책 테스트 확장.

## Key State

- 감지 계층: tier1(para_shape) → tier2(style) → tier3(font+bold) → tier4(text pattern)
- Tier-4 매핑: 편/장→H1, 절/조→H2. 관 연기.
- 모듈: heading_style.rs (extract from convert.rs, shared with hwpx)
- Test count: 1240 lib + 31 integration (2026-05-22)
- Next: Sprint 70 (관 검토, flush 헬퍼 추출)

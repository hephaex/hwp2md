# hwp2md — Progress

## 현재 상태: Phase 0 (프로젝트 초기화)

### 완료

- [x] 기술 조사 — 기존 Rust 크레이트 현황 파악 (unhwp, hwpforge, hwpers 등)
- [x] 기존 도구 조사 — 타 언어 HWP 변환 도구 전수 조사
- [x] HWP/HWPX 포맷 구조 분석
- [x] 아키텍처 설계 — IR 기반 양방향 변환 파이프라인
- [x] 의존성 선정 — unhwp (HWP→MD), hwpforge (MD→HWPX)
- [x] 프로젝트 초기화 — Cargo.toml, 모듈 구조, CLI 스켈레톤
- [x] GitHub 저장소 생성 — github.com/hephaex/hwp2md (GPLv3, public)
- [x] PLAN.md / PROGRESS.md 작성

### 진행 중

없음

### 미착수

- [ ] Phase 1: HWP 5.0 → Markdown (unhwp 래핑)
- [ ] Phase 2: HWPX → Markdown
- [ ] Phase 3: Markdown 렌더러 고도화
- [ ] Phase 4: Markdown → IR 파서
- [ ] Phase 5: IR → HWPX 라이터 (hwpforge)
- [ ] Phase 6: CLI 완성 + 배포

## 변경 이력

### 2026-04-21 — 프로젝트 초기화

- 기술 조사 완료 (Rust HWP 생태계, 기존 도구, 포맷 스펙)
- 핵심 발견: `unhwp` (HWP/HWPX→MD 기 구현), `hwpforge` (MD→HWPX 코덱 보유)
- 프로젝트 scaffolding: 8 모듈 (lib, main, convert, ir, error, hwp_reader, hwpx_reader, md_writer, md_parser, hwpx_writer)
- CLI: clap derive 기반 (to-md, to-hwpx, info 서브커맨드)
- IR 설계: Document → Section → Block → Inline 계층, Asset 분리

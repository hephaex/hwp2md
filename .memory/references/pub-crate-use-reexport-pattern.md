# Rust: pub(crate) use re-export for test visibility

> 테스트 서브모듈에서 `use super::*`로 접근할 함수를 다른 모듈에서 가져올 때 필요한 패턴.

---

## 문제

`convert.rs`에 있던 함수를 `heading_style.rs`로 추출했을 때,
`convert_tests_detect.rs`의 `use super::*`가 해당 함수를 더 이상 볼 수 없게 됨.

```
error[E0425]: cannot find function `is_heading_terminator` in this scope
```

`#[allow(unused_imports)] use crate::hwp::heading_style::is_heading_terminator;` 는
re-export가 아니므로 `use super::*`로는 노출되지 않음.

## 해결 방법

두 가지 옵션:

**옵션 A**: `convert.rs`에서 `pub(crate) use`로 재수출
```rust
pub(crate) use crate::hwp::heading_style::detect_korean_regulation_heading;
```
→ `use super::*`가 `detect_korean_regulation_heading`을 가져올 수 있음.
→ 단, CI의 `-D warnings`에서 "unused import" 오류가 날 수 있음
  (`pub(crate) use`로 선언해도 production 코드에서 직접 안 쓰면 경고).

**옵션 B**: 테스트 파일에서 직접 import (권장)
```rust
// convert_tests_detect.rs
use super::*;
use crate::hwp::heading_style::is_heading_terminator;  // 직접 import
```
→ `is_heading_terminator`처럼 production `convert.rs`에서 직접 안 쓰는 함수는
  테스트 파일에서 직접 import하면 unused import 경고 없음.

**이번 해결**: 옵션 A + B 병행
- `detect_korean_regulation_heading`: `pub(crate) use` (convert.rs에서 실제로 호출하므로 unused 아님)
- `is_heading_terminator`: 테스트 파일에서 직접 import

## 주의사항

- CI에 `cargo clippy -- -D warnings`가 있으면 반드시 확인
- `pub(crate) use`는 production 모듈 코드에서 실제로 사용되지 않으면
  `-D warnings`에서 "unused import" 오류 발생
- `use super::*`는 `pub(crate)` 또는 `pub` 가시성 아이템만 가져옴

## 관련 문서

- Sprint 69 에피소드: `.memory/episodes/daily/2026-05-22.md`

---
*작성: 2026-05-22*

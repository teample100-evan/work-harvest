# Work Harvest macOS 배포

Work Harvest는 우선 Apple Silicon용 앱과 native `wh` CLI를 함께 배포한다. Tauri 앱에는 `Contents/MacOS/wh` sidecar가 포함되며, 같은 GitHub Release에 standalone CLI tarball과 SHA-256 파일도 게시한다.

## 로컬 패키징

```bash
pnpm install
pnpm desktop:build
pnpm desktop:bundle:verify
```

`desktop:build`는 다음을 한 번에 수행한다.

1. `work-harvest-cli`를 현재 Rust host target의 release 모드로 빌드한다.
2. Tauri가 요구하는 `wh-<target-triple>` 이름으로 생성 디렉터리에 복사한다.
3. React 프론트엔드와 Tauri 앱을 빌드한다.
4. native CLI가 포함된 `.app`과 DMG를 만든다.

생성한 target별 sidecar는 소스가 아니므로 Git에서 제외한다. 기본 Apple Silicon 산출물은 다음 위치에 있다.

```text
target/release/bundle/macos/Work Harvest.app
target/release/bundle/dmg/Work Harvest_0.1.0_aarch64.dmg
```

로컬 빌드는 Developer ID 인증서가 없으면 배포 가능한 서명·공증 상태가 아니다. `desktop:bundle:verify`는 bundled CLI의 실행과 데이터 검증을 확인하고 현재 서명 정보를 출력한다.

## GitHub Release 준비

`.github/workflows/release-macos.yml`은 `v<version>` 태그가 push될 때만 실행한다. 다음 GitHub Actions secret이 모두 필요하다.

| Secret | 용도 |
| --- | --- |
| `APPLE_CERTIFICATE` | Developer ID Application `.p12`의 base64 본문 |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` 내보내기 암호 |
| `APPLE_ID` | Apple 공증 계정 |
| `APPLE_PASSWORD` | Apple app-specific password |
| `APPLE_TEAM_ID` | Apple Developer Team ID |

인증서는 저장소에 파일로 추가하지 않는다. 워크플로는 임시 keychain에 인증서를 import하고 다음 조건을 모두 검사한다.

- 태그와 root·desktop·Tauri·Rust CLI 버전 일치
- 전체 Node·React·Rust 회귀 테스트
- standalone `wh`의 Developer ID hardened-runtime 서명
- bundled `wh` 실행과 예제 데이터 검증
- 앱의 Developer ID 서명
- Apple 공증과 Tauri 번들 생성

성공하면 GitHub 초안 Release에 앱·DMG와 다음 CLI 파일을 함께 올린다.

```text
work-harvest-cli-v<version>-aarch64-apple-darwin.tar.gz
work-harvest-cli-v<version>-aarch64-apple-darwin.tar.gz.sha256
```

초안의 앱 설치, Gatekeeper, `wh validate`와 실제 데이터 읽기 비교를 확인한 뒤 수동으로 공개한다.

## 버전 발행

버전은 다음 다섯 곳에서 같아야 한다.

- `package.json`
- `apps/desktop/package.json`
- `apps/desktop/src-tauri/tauri.conf.json`
- `apps/desktop/src-tauri/Cargo.toml`
- `crates/work-harvest-cli/Cargo.toml`

태그를 만들기 전에 검사한다.

```bash
node scripts/verify-release-version.mjs v0.1.0
git tag v0.1.0
git push origin v0.1.0
```

## 자동 업데이트와 Node fallback

자동 업데이트에는 Apple 인증서와 별도로 Tauri updater 서명 키가 필요하다. 개인키는 GitHub secret으로만 보관하고 public key를 앱 설정에 추가한 뒤 updater 플러그인과 `latest.json` 생성을 활성화한다. 키가 아직 발급되지 않았으므로 현재 워크플로는 자동 업데이트 산출물을 만들지 않는다.

Node 호환 CLI는 첫 서명 릴리스의 rollback 경로다. 다음 조건을 모두 충족한 후 제거한다.

1. 초안 Release의 앱과 standalone CLI가 Gatekeeper 검사를 통과한다.
2. 실제 사용자 데이터 복사본에서 Node·Rust read-only 결과가 일치한다.
3. 설치 앱만 있는 환경에서 Codex Skill이 bundled CLI로 기록과 검증을 완료한다.
4. 실사용 기간에 native CLI 때문에 Node fallback으로 복귀한 사례가 없다.

Universal binary는 Intel Mac 지원이 실제로 필요해질 때 두 target을 빌드하고 `lipo`로 CLI sidecar를 합치는 별도 단계로 추가한다.

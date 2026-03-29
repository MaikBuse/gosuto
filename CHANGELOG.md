# Changelog

## [0.11.1](https://github.com/MaikBuse/gosuto/compare/v0.11.0...v0.11.1) (2026-03-29)


### Bug Fixes

* select SFU focus from existing call participants in federated rooms ([d0ae70b](https://github.com/MaikBuse/gosuto/commit/d0ae70b1281de6f18df353b8cc81b99ade52e9a8))

## [0.11.0](https://github.com/MaikBuse/gosuto/compare/v0.10.0...v0.11.0) (2026-03-15)


### Features

* new logo with transparant background ([4d79410](https://github.com/MaikBuse/gosuto/commit/4d7941092e92f37a64ebe16fa3fd2ab867d582b6))


### Bug Fixes

* also check encryption_keys PL in ensure_call_member_permissions ([16269a7](https://github.com/MaikBuse/gosuto/commit/16269a7e82b17f0670b4e268784cd00c9f1ce86e))
* skip remove_encryption_keys on leave when keys were never published ([82efd1d](https://github.com/MaikBuse/gosuto/commit/82efd1dd4982abcf63222cd7aec748e5bae9fabf))

## [0.10.0](https://github.com/MaikBuse/gosuto/compare/v0.9.0...v0.10.0) (2026-03-14)


### Features

* add MSIX packaging and Microsoft Store publishing to release workflow ([a4768b3](https://github.com/MaikBuse/gosuto/commit/a4768b367d24aebdee20b6281b429173890fc0f9))
* add one-off MSIX build workflow for manual Store submission ([a5b60ee](https://github.com/MaikBuse/gosuto/commit/a5b60ee8e46494fe3e33b0138f6dabae74c4ed55))
* update demo video ([67b12d6](https://github.com/MaikBuse/gosuto/commit/67b12d634f3b2c29af9e4e12779d41e9338a815f))
* upload MSIX to GitHub release and conditionally publish to Store ([59d65d6](https://github.com/MaikBuse/gosuto/commit/59d65d6c0c512dd2b841c1b2feddd25c686e67d8))


### Bug Fixes

* add visual line wrapping in insert mode input bar ([a8af68d](https://github.com/MaikBuse/gosuto/commit/a8af68d16189fcca09a668236ace938c24eee658))
* prefix source paths in MSIX mapping.txt to resolve from repo root ([9f4c5c5](https://github.com/MaikBuse/gosuto/commit/9f4c5c567bbe97daf44516582652dd4a2a3c49f4))
* query fresh device keys before encrypting VoIP key exchange ([58a286b](https://github.com/MaikBuse/gosuto/commit/58a286b29b66203fcc9e740b9b7bdafab37a7dc4))

## [0.9.0](https://github.com/MaikBuse/gosuto/compare/v0.8.0...v0.9.0) (2026-03-13)


### Features

* add cursor movement, forward delete, and Ctrl+J newline to input editor ([8c7cab3](https://github.com/MaikBuse/gosuto/commit/8c7cab3ae6ce78a3ce14e1a8c3516f142fabcdb6))
* add lazy loading to the chat pane ([f87293d](https://github.com/MaikBuse/gosuto/commit/f87293db200403374e953a75ebe5b09eac3b9877))
* add lazy-loading pagination and use typed MatrixRTC events ([42bf01b](https://github.com/MaikBuse/gosuto/commit/42bf01b31cd4b2a672f511b34b462ba9610c45c2))
* add markdown formatting for outgoing messages ([d80d8cd](https://github.com/MaikBuse/gosuto/commit/d80d8cd0c7cf0e50ae61c83982a79c72212cda3e))
* add message editing with e key and fix multi-edit persistence ([a195aea](https://github.com/MaikBuse/gosuto/commit/a195aea9ee218f565c2ff9597712c7af0e1719fd))
* add message redaction with d key and confirmation prompt ([289adda](https://github.com/MaikBuse/gosuto/commit/289addaa9ec616ec8e2e23ffc6ea5035f44f7384))
* default login focus to homeserver and split hint lines ([cb76c78](https://github.com/MaikBuse/gosuto/commit/cb76c7827bdc76f89db12c798b1d225783b1db19))
* embed ghost icon in Windows executable ([86d683f](https://github.com/MaikBuse/gosuto/commit/86d683fe12c1625c49898d0231943832a85567fd))
* replace chat matrix rain with one-shot message rain effect ([cd91249](https://github.com/MaikBuse/gosuto/commit/cd91249c825efcbdb31a4ece102be6d65a2a8223))
* retheme UI to Tokyo Night and equalize sidebar widths ([e9d0be7](https://github.com/MaikBuse/gosuto/commit/e9d0be7e56d1055519cc19152bc4d7e6a59e5e52))
* show matrix-style katakana characters during message rain fall ([f9e45b5](https://github.com/MaikBuse/gosuto/commit/f9e45b5030254deabaaa3370d27d519a8ea10f29))
* smooth color transitions between call overlay states ([1109f46](https://github.com/MaikBuse/gosuto/commit/1109f468a85c7d6d23737f09e55c24dac3c86fbb))


### Bug Fixes

* brighten date separators and remove dead pulse animation ([5a94c70](https://github.com/MaikBuse/gosuto/commit/5a94c70711578e80089adefccc8f732ccd4846ee))
* keep existing messages visible during partial rain effect ([005c537](https://github.com/MaikBuse/gosuto/commit/005c537d36cf834ff2d45e66a73b3775b95892cf))
* remove unused variables ([2459a3c](https://github.com/MaikBuse/gosuto/commit/2459a3cbcd510349c7ea44f80156e7728033df08))
* suppress native library stdout corruption and add real connection phases ([96aeffd](https://github.com/MaikBuse/gosuto/commit/96aeffda95b8c71a5d96eabcded653de20daab6a))
* use raw spans for indent spaces so matrix rain shows through ([2bfecbf](https://github.com/MaikBuse/gosuto/commit/2bfecbfbfd1e5a139902f1e33556d1b3fef1a33e))
* wrap dup'd fd in BufWriter to eliminate cursor flicker ([777c734](https://github.com/MaikBuse/gosuto/commit/777c734b916b41f42fae3d73d8e6aa5a8fa9fc65))

## [0.8.0](https://github.com/MaikBuse/gosuto/compare/v0.7.0...v0.8.0) (2026-03-10)


### Features

* add gradient color system and animated UI effects ([b38594a](https://github.com/MaikBuse/gosuto/commit/b38594a7eed95b9154b1426d6817ea82b1b5a3e9))
* add release build checks for Linux and Windows to CI ([6af0967](https://github.com/MaikBuse/gosuto/commit/6af09677b09002b6c45b79e02b5aeabf0993b8e6))
* switch to gosuto-livekit fork with configurable HKDF key derivation ([77c2798](https://github.com/MaikBuse/gosuto/commit/77c2798f21fe24766ed2f7afee751e62b2b709a4))
* xy ([e9e2abb](https://github.com/MaikBuse/gosuto/commit/e9e2abb33235bfb321b61e6b86a55d565b461f44))
* xy ([157af6f](https://github.com/MaikBuse/gosuto/commit/157af6f1d763577dab73274b711b945bfc7dc393))
* xy ([157af6f](https://github.com/MaikBuse/gosuto/commit/157af6f1d763577dab73274b711b945bfc7dc393))
* xy ([157af6f](https://github.com/MaikBuse/gosuto/commit/157af6f1d763577dab73274b711b945bfc7dc393))


### Bug Fixes

* Olm-encrypt to-device key exchange and fix keys format for Element X ([b5ba59f](https://github.com/MaikBuse/gosuto/commit/b5ba59f05802f2ee4431fd16a21cd71b2c4415f1))
* show DM room display name consistently even when it matches localpart ([91c41ea](https://github.com/MaikBuse/gosuto/commit/91c41ea4cc62c6a4c9c2d5a8202fcfd8af616042))
* update gosuto-livekit to 0.7.33 for Windows MAX_PATH fix ([878e7c7](https://github.com/MaikBuse/gosuto/commit/878e7c76bbe955561fcfb7962ae0d242e3856318))
* update gosuto-livekit to 0.7.34 for Windows MAX_PATH fix ([afa2fea](https://github.com/MaikBuse/gosuto/commit/afa2feab287f591ddd293ce59a524dd546355793))
* update gosuto-livekit to 0.7.35 for Windows MAX_PATH fix ([3994577](https://github.com/MaikBuse/gosuto/commit/39945779b6c45754804b842ea3248bffa8a78d20))
* use Swatinem/rust-cache in release workflow to cache compiled artifacts ([9c7a3da](https://github.com/MaikBuse/gosuto/commit/9c7a3da6b085ec8bb8ee8e6605e4cc6d2cd44286))

## [0.7.0](https://github.com/MaikBuse/gosuto/compare/v0.6.0...v0.7.0) (2026-03-09)


### Features

* add reply to message support ([cbf6bcc](https://github.com/MaikBuse/gosuto/commit/cbf6bcc53dbb9e99fceda88b314bf87b0975e833))
* load historical reactions and redesign reaction picker ([8f8d274](https://github.com/MaikBuse/gosuto/commit/8f8d274818562b5ba4965999e8c873c001caeda7))


### Bug Fixes

* resolve Windows build errors in global_ptt and fs_utils ([21107d4](https://github.com/MaikBuse/gosuto/commit/21107d47b01158e4d778cc9274a9bd494661795e))

## [0.6.0](https://github.com/MaikBuse/gosuto/compare/v0.5.0...v0.6.0) (2026-03-09)


### Features

* add --profile flag for multi-instance support ([8baad5f](https://github.com/MaikBuse/gosuto/commit/8baad5f9b276e75838c6aa92b64810edeb027479))
* add action menu to :verify command ([bdcf9c6](https://github.com/MaikBuse/gosuto/commit/bdcf9c60850a38088b5cf0352664e57b385fa598))
* add loading indicator for room list during initial sync ([8df8334](https://github.com/MaikBuse/gosuto/commit/8df8334d7a2897840c9af72d4b31d335d0b30384))
* add members pane tooltip and improve DM name format ([9cfd8e8](https://github.com/MaikBuse/gosuto/commit/9cfd8e80a3fc3a3b71a4209d548137c49bbb63ff))
* add message selection mode to Messages panel ([5209c29](https://github.com/MaikBuse/gosuto/commit/5209c29b11215b3644ec8f0f1bc5488d2451cbe9))
* add room invitation support ([9cb4bf3](https://github.com/MaikBuse/gosuto/commit/9cb4bf3372241e45a4a22138599ed220adeeb6d8))
* improve recovery status granularity, DM naming, and member display ([5bd1626](https://github.com/MaikBuse/gosuto/commit/5bd162667d7744b796a2ec875c07cfae99e840df))


### Bug Fixes

* abort orphaned verification tasks on cancellation ([8c2c9df](https://github.com/MaikBuse/gosuto/commit/8c2c9df675c45f04a094c4eaf2e896dd9b4a5554))
* force-exit to prevent rdev listener thread from lingering after shutdown ([b47121c](https://github.com/MaikBuse/gosuto/commit/b47121c6dbd9978e716dafc7baecaf483eb387cc))
* handle in-room verification requests from other users ([f2bd58c](https://github.com/MaikBuse/gosuto/commit/f2bd58c47b376b3ad4f5a00b901b89c71a8e368e))
* pin CI workflows to rust 1.93.1, fix verification and clippy lints ([ad7f2c4](https://github.com/MaikBuse/gosuto/commit/ad7f2c4f2d023f6701cba965aa19be5e8a19e4b4))
* replace panicking unwrap/expect calls with proper error handling ([88b4acd](https://github.com/MaikBuse/gosuto/commit/88b4acd046df67fc7a90f779a67ad6321db1017d))
* replace rdev::grab() with passive evdev listener to prevent stuck keys ([112db45](https://github.com/MaikBuse/gosuto/commit/112db457958303d7df961bcfe5f022ad3165462a))
* restrict file permissions and warn on insecure connections ([727938b](https://github.com/MaikBuse/gosuto/commit/727938be7fa12b63b3a0660bc22b6aed9c3550cf))
* scope debug logging to gosuto, suppress noisy dependencies ([6553edd](https://github.com/MaikBuse/gosuto/commit/6553edd871bcdc670606c2352349d654dc7f99c9))
* suppress transmission popup flash during initial sync ([6880980](https://github.com/MaikBuse/gosuto/commit/68809806cc10d0df381c475259af21bf943aadba))
* sync audio config to CallManager when settings change ([ef38696](https://github.com/MaikBuse/gosuto/commit/ef38696fef1f10d524f9cbfacb65ca6fd7913f46))

## [0.5.0](https://github.com/MaikBuse/gosuto/compare/v0.4.0...v0.5.0) (2026-03-08)


### Features

* add voice transmission indicator in status bar ([85a550e](https://github.com/MaikBuse/gosuto/commit/85a550efb33c21eb609ff09090763635d586cc5e))


### Bug Fixes

* **ci:** fix release binaries workflow for Linux and Windows ([b5a92ac](https://github.com/MaikBuse/gosuto/commit/b5a92acffc4ca3ac413ce96d6006a16b754816e6))

## [0.4.0](https://github.com/MaikBuse/gosuto/compare/v0.3.0...v0.4.0) (2026-03-08)


### Features

* add change password popup with :password command ([49c92e2](https://github.com/MaikBuse/gosuto/commit/49c92e296ce22582c6c4b47b5bbc0566e09a3573))
* add demo mode for offline UI exploration ([11a40d6](https://github.com/MaikBuse/gosuto/commit/11a40d69944fe20d08d564ea5b26f4ce404d1965))
* add ellipsis and tooltip for truncated room names ([c705da7](https://github.com/MaikBuse/gosuto/commit/c705da7165f7ad2050a8d6465cc351149b7a825c))
* add global push-to-talk via rdev for cross-app PTT support ([ac73ab0](https://github.com/MaikBuse/gosuto/commit/ac73ab02a91c0ababf632044a7c616fca2de3e97))
* add inline image display with background encoding ([4cb40b9](https://github.com/MaikBuse/gosuto/commit/4cb40b9cb9bb896c9dc7f73d80414f279a9732c6))
* add multi-line chat message support with Alt+Enter ([6455f53](https://github.com/MaikBuse/gosuto/commit/6455f53872b808c42c4c9b1b09c30d42300fbf44))
* add Nerd Font icons with :nerdfonts toggle command ([2aaf07e](https://github.com/MaikBuse/gosuto/commit/2aaf07eee4a72875dfbd58d9acd87bbebe69b46f))
* add recovery modal with healing for incomplete accounts ([c34445d](https://github.com/MaikBuse/gosuto/commit/c34445d4418bf9004dd945b246cf998ddc457d1a))
* add typing indicators (send and receive) ([4aff290](https://github.com/MaikBuse/gosuto/commit/4aff2903156b0dcba8d0ad7e8fcfce589e1bee64))
* format code ([285890e](https://github.com/MaikBuse/gosuto/commit/285890eb46d3a356924887acdee90d49fbbaee58))
* improve PTT listener resilience and error reporting ([56cea82](https://github.com/MaikBuse/gosuto/commit/56cea822ed60daf5e544b2dead1c36a6fae7ef2d))
* remove verification and recovery features for clean slate ([93c2268](https://github.com/MaikBuse/gosuto/commit/93c226878a47a1a35e5196a0997a8fe378e01db7))
* rename :configure to :profile and show security status ([c810105](https://github.com/MaikBuse/gosuto/commit/c8101056e839204d10cd1aca7f99cfe09e3143ea))
* restore original verification implementation from before 93c2268 ([5100e81](https://github.com/MaikBuse/gosuto/commit/5100e818badab16e25f0a40afded970c8d029feb))
* rewrite demo chat messages and add demo video to README ([cb2347e](https://github.com/MaikBuse/gosuto/commit/cb2347e924451240a7c4ac325b4185bffe6d8ecb))
* support modifier-only keys (Ctrl, Shift, Alt) as PTT keys ([bdf9639](https://github.com/MaikBuse/gosuto/commit/bdf9639af0e5545ef4e649e84707eb80230ab9d5))


### Bug Fixes

* apply sqrt scaling to mic level meter for better visibility ([25d9f4a](https://github.com/MaikBuse/gosuto/commit/25d9f4a5fc55ea6cc6ddcb04b424ea99a545a509))
* **ci:** add libx11-dev to system dependencies ([c70eb78](https://github.com/MaikBuse/gosuto/commit/c70eb78dda6bfad99fdc81b8b2add951844127d2))
* **ci:** add libxi-dev and libxtst-dev for x11 crate features ([312379e](https://github.com/MaikBuse/gosuto/commit/312379ead0532851727a11d8114c03dc1d7f4801))
* clear persistent unread badge on encrypted DM rooms ([285890e](https://github.com/MaikBuse/gosuto/commit/285890eb46d3a356924887acdee90d49fbbaee58))
* download room keys from backup after recovery to fix undecryptable messages ([2e7250e](https://github.com/MaikBuse/gosuto/commit/2e7250e15ff5f87d949c174e54204971265f4761))
* harden VoIP E2EE key handling and extract shared helpers ([880a779](https://github.com/MaikBuse/gosuto/commit/880a779ef5c75c1e5df262e438c8020705bc3ac0))
* make encryption key publish non-fatal for PL 0 users ([ff59f20](https://github.com/MaikBuse/gosuto/commit/ff59f20404b7f033d3323871ee866d091b8fc4e2))
* make voice activity and push-to-talk mutually exclusive ([76b802c](https://github.com/MaikBuse/gosuto/commit/76b802c9f60d45761eb140c87cc6e4124712a40e))
* persist recovery state and mark user as verified after recovery ([78edf03](https://github.com/MaikBuse/gosuto/commit/78edf03cc7b3c0f3bb04b5b496aeaad873d8b306))
* pin Rust toolchain to 1.93.1 to avoid matrix-sdk query depth overflow ([8e42b77](https://github.com/MaikBuse/gosuto/commit/8e42b77f8cfc5face4c3c2a638d91792895683e6))
* prevent effects from overwriting ratatui-image skip cells ([6468486](https://github.com/MaikBuse/gosuto/commit/64684864312613fa0dff2ce30b36352b4d693631))
* replace needs_resize with deterministic rect tracking to eliminate per-frame re-encoding ([1b79b38](https://github.com/MaikBuse/gosuto/commit/1b79b382ef5aba2ea6e045ff4e702c643626ad08))
* resolve clippy warnings for MSRV and collapsible if ([c301b4e](https://github.com/MaikBuse/gosuto/commit/c301b4efc51dbb69e5c386eba36e4bf264c6f4cc))
* treat Incomplete recovery state as enabled and remove diagnostic logging ([e775f70](https://github.com/MaikBuse/gosuto/commit/e775f70e6d80c56822392dc6c84632ea719900d6))
* use evdev backend for PTT on Wayland ([f1723ce](https://github.com/MaikBuse/gosuto/commit/f1723ce48afc143e3236b6c11f1d84dd28c7a265))

## [0.3.0](https://github.com/MaikBuse/gosuto/compare/v0.2.0...v0.3.0) (2026-03-02)


### Features

* add which-key leader popup, room list domain grouping, and call participants ([92bb578](https://github.com/MaikBuse/gosuto/commit/92bb578b0853b448d7cb796fa40e9b41f3056206))
* make topic editable in room edit modal ([e9a6c7d](https://github.com/MaikBuse/gosuto/commit/e9a6c7d8282c8aa8d5cb94f9df33410ad959a881))
* replace :create command with interactive create room modal ([60bcc15](https://github.com/MaikBuse/gosuto/commit/60bcc154b86a27575941385801ad70bbec61394e))


### Bug Fixes

* **ci:** bundle SQLite to fix Windows linking failure ([17ccb71](https://github.com/MaikBuse/gosuto/commit/17ccb719d912c56b24baf3a1d341471b1f237a8b))
* resolve clippy warnings in which_key.rs ([81afc8c](https://github.com/MaikBuse/gosuto/commit/81afc8c534e899c392469fb8ea5040d361c6e50e))
* show UTD placeholders and re-fetch messages after verification ([5ce0af8](https://github.com/MaikBuse/gosuto/commit/5ce0af86c886c5e76ce5a8287a85e990d8e20ed7))


### Performance Improvements

* **ci:** parallelize pipeline and improve caching ([2edda45](https://github.com/MaikBuse/gosuto/commit/2edda456d11d4a564c2e7fed4e139056cc8647e0))

## [0.2.0](https://github.com/MaikBuse/gosuto/compare/v0.1.1...v0.2.0) (2026-03-01)


### Features

* add confirmation step before recovery key reset ([b3ef4ce](https://github.com/MaikBuse/gosuto/commit/b3ef4cedca25d2c59f2cac6a960a6102dca28587))
* add descriptions for history visibility options in :edit screen ([2d641ee](https://github.com/MaikBuse/gosuto/commit/2d641ee005a19df54b7799e71373aeca442ba4b5))
* add device verification with cross-signing bootstrap and SAS emoji flow ([0199f85](https://github.com/MaikBuse/gosuto/commit/0199f851567721a6e1a23e5688cc301fd1e73296))
* add full-pane EMP shockwave with cyberpunk orange and members pane effect ([38406bc](https://github.com/MaikBuse/gosuto/commit/38406bc31020008307f1b913f7adfa008f3ed856))
* add recovery key import flow for restoring encrypted history ([63d34de](https://github.com/MaikBuse/gosuto/commit/63d34de88c721bfeccea8ce3e4b20b82c4fea516))
* add recovery key management with modal UI and create/reset flow ([55f85ed](https://github.com/MaikBuse/gosuto/commit/55f85edf6b0a08742f3f1490ae4a5db626c247c2))
* add room selection pane v2.0 with space hierarchy and animated highlights ([94b0269](https://github.com/MaikBuse/gosuto/commit/94b026989466e08516f823656c06ad4a7248450c))
* allow enabling encryption on existing unencrypted rooms from room info ([a262be1](https://github.com/MaikBuse/gosuto/commit/a262be11e9e32e127771ff257a626b5faad15c0f))
* enable E2EE by default for new rooms and show encryption status in room info ([f8257e0](https://github.com/MaikBuse/gosuto/commit/f8257e03df1e69cea05929be2477310625bd8ff2))
* enable rustls TLS for LiveKit WebSocket connections ([eeeb4e0](https://github.com/MaikBuse/gosuto/commit/eeeb4e05a143061fd4395fb9ffc198c9f43f5abf))
* glitch effect ([f58e88d](https://github.com/MaikBuse/gosuto/commit/f58e88df336e8ec8ed64b7a7de86bcd5e5ea44ce))
* implement incoming call detection, room name display, and VoIP improvements ([6bacbd9](https://github.com/MaikBuse/gosuto/commit/6bacbd9b8d7ae2a0481e2c07d470b192878630da))
* implement MatrixRTC SFrame E2EE for audio calls ([c9d5d7b](https://github.com/MaikBuse/gosuto/commit/c9d5d7b8a743bd0414f36e314195d7a6cd8b29e6))
* rename :info to :edit and add :configure command for user profile ([5de1506](https://github.com/MaikBuse/gosuto/commit/5de15068b96bd4cf20359b422eb799879d25806a))
* replace linear resampler with sinc resampler and move capture DSP off real-time thread ([4e2034d](https://github.com/MaikBuse/gosuto/commit/4e2034deb4341f3c7cd7993072b981d6291b7b77))
* rework logging with XDG log dir, 7-day cleanup, and JWT redaction ([dd95fa1](https://github.com/MaikBuse/gosuto/commit/dd95fa189380575d4c0e9125f0a0e797ec206c7c))


### Bug Fixes

* align call member events to Element X and fix clippy warnings ([e7e9549](https://github.com/MaikBuse/gosuto/commit/e7e9549305f8d4d7f2031789598f8e24d4fd9ce4))
* apply rustfmt formatting and use request_user_identity API ([2c1c252](https://github.com/MaikBuse/gosuto/commit/2c1c25271599e3e8b256d9dc06eacf477d1dd3ac))
* call notification to use MSC4075 rtc.notification so Element X rings ([3e3befa](https://github.com/MaikBuse/gosuto/commit/3e3befa7019f0991f3360acc04f6cf498dae1687))
* change EMP pulse effect from orange to magenta/purple ([61f2ac2](https://github.com/MaikBuse/gosuto/commit/61f2ac2bd0e5a23c29e077c5982447a3aeed0abf))
* **ci:** add missing libva-dev dependency for webrtc-sys build ([890eab3](https://github.com/MaikBuse/gosuto/commit/890eab3b4a563f1e708075742ce1d409818fed8a))
* clipboard error message corrupting TUI on Linux ([9b0cf80](https://github.com/MaikBuse/gosuto/commit/9b0cf80a07235da58ad1665d9211915ae4d74655))
* collapse nested if blocks to satisfy clippy::collapsible_if ([0d154d7](https://github.com/MaikBuse/gosuto/commit/0d154d74ae0c0d1eff4c2d2bd6f6459326ba7d42))
* decouple binary builds from release-please and harden CI pipeline ([d7e1ac3](https://github.com/MaikBuse/gosuto/commit/d7e1ac3c0c9204c7925f81db4edf17820a3f9c9b))
* fetch E2EE encryption keys directly from server instead of empty local store ([7db3d57](https://github.com/MaikBuse/gosuto/commit/7db3d573bea11027dbc2e323f81708a058eb18d2))
* handle incomplete recovery state and show full errors in modal ([1282ab5](https://github.com/MaikBuse/gosuto/commit/1282ab5ade6564516621089801618717f7eba013))
* handle non-f32 sample formats in audio capture and playback ([1cf628e](https://github.com/MaikBuse/gosuto/commit/1cf628e2e421b999d4bab88494e132f0a7a03cd5))
* keep focus on room list after selection and exit insert mode on send ([d1843ff](https://github.com/MaikBuse/gosuto/commit/d1843ff82edec698c7ff89322b3a2c07c5ce2a26))
* LiveKit VoIP access_token query param, endpoint fallback, and diagnostics ([5565375](https://github.com/MaikBuse/gosuto/commit/55653751f7aa23367bb8e0d30cf0282cbf1a1f83))
* persist verification status across :configure reopens ([9098991](https://github.com/MaikBuse/gosuto/commit/9098991c19e07e6bf91acdabde697554582ff419))
* release binary builds not triggering ([fc8f60e](https://github.com/MaikBuse/gosuto/commit/fc8f60e82dc2f67ff2617bd2ea4484491da5415c))
* remove focus-based dimming of EMP pulse background effects ([944fa3f](https://github.com/MaikBuse/gosuto/commit/944fa3fb7230093a00e334479ef655a0fe0396c7))
* remove target/ from CI cache to prevent stale build artifacts ([ce0186b](https://github.com/MaikBuse/gosuto/commit/ce0186b232243ea15dd61bc8abef4f55a72ee938))
* shorten history visibility descriptions to fit modal width ([baf580c](https://github.com/MaikBuse/gosuto/commit/baf580c19921e19c2c609f4b1445f933d6b246fe))
* swapped send_state_event_raw arguments causing server rejection ([b89c7a1](https://github.com/MaikBuse/gosuto/commit/b89c7a1d6f973c567d12f2646c5d3f1d41929aa7))
* transmission popup room name visibility and waveform width ([e4eccf9](https://github.com/MaikBuse/gosuto/commit/e4eccf99ed0f8966d508d6d92efd8d4fda8a339e))
* tweak text reveal scramble characters ([cb69914](https://github.com/MaikBuse/gosuto/commit/cb69914e6eeb59d1a4cb2d201ddd3fc1813651c7))
* use per-participant E2EE keys and match by LiveKit identity ([2664405](https://github.com/MaikBuse/gosuto/commit/266440561e24545cf32a6d2115ee648c20dcc4ef))

## [0.1.1](https://github.com/MaikBuse/gosuto/compare/gosuto-v0.1.0...gosuto-v0.1.1) (2026-02-28)


### Features

* glitch effect ([f58e88d](https://github.com/MaikBuse/gosuto/commit/f58e88df336e8ec8ed64b7a7de86bcd5e5ea44ce))

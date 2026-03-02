# Changelog

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

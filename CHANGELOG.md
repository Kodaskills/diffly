
## v0.2.0 — 2026-02-20


### ✨ Features
- Add viper-style layered configuration loading ([f3da724](https://github.com/kodaskills/diffly/commit/f3da7240749f9a879c9ba8f1f1f7a1b497f606a9))
- Embed PerfReport in JSON and HTML outputs ([0c3fa47](https://github.com/kodaskills/diffly/commit/0c3fa475f92e3eb7d2515e27e12ba17197c562b0))


### 🐛 Bug Fixes
- Percent-encode a string for safe use in a connection URL ([6663d90](https://github.com/kodaskills/diffly/commit/6663d90565b35691002aabe926589e350bd477c0))
- Render PK columns first in declared order for all change groups ([a929c07](https://github.com/kodaskills/diffly/commit/a929c07f0bef308e3aeb29e9a438bd16eccdc32a))


### 👷 CI
- Default diff.tables to [] so env-only configs pass on CI ([6959ee3](https://github.com/kodaskills/diffly/commit/6959ee34d9ab965a122c563526b8fcfd09b73538))

## v0.1.0 — 2026-02-19


### ✨ Features
- Init project ([bad12e7](https://github.com/kodaskills/diffly/commit/bad12e7180618f979968e63b968e6fe4209db73a))
- Implements the 3 way merge with snapshot and check-configs commands ([ae17070](https://github.com/kodaskills/diffly/commit/ae170707152b835da0f8b4bc2008c4b117446586))
- Add snapshots into configoutput dit with clear driver and timestamp isolation ([66a862c](https://github.com/kodaskills/diffly/commit/66a862c2a35befae9db0615f8dfb1b7c9b7912b0))


### 👷 CI
- Add write permissions ([5745ad4](https://github.com/kodaskills/diffly/commit/5745ad487cf9404e747f136f5c32c92b3a6a10b3))
- Add auto CHANGELOG for tags and use of env vars for repeated names ([733e20c](https://github.com/kodaskills/diffly/commit/733e20c41b5f8c889c755099a5dd31116764dcb6))
- Fix auto CHANGELOG for tags ([c288845](https://github.com/kodaskills/diffly/commit/c288845d8b76523d557362d321ae110aee87085c))
- Fix bad closing sequences ([11ef656](https://github.com/kodaskills/diffly/commit/11ef656156491ab2c47531ae3ab78da00a297706))
- Fix env not allowed in matrix ([77e85f5](https://github.com/kodaskills/diffly/commit/77e85f5fb4333894ad3112563a2d787527b3a06e))
- Fix auto changelog tag ([8523f54](https://github.com/kodaskills/diffly/commit/8523f54d19ed7295e7de14662865f5b915c4e319))
- Couldn't find a previous tag ([6e5e136](https://github.com/kodaskills/diffly/commit/6e5e136286ed34392d6209d0f36b229382af395e))
- MacOS bin can't be a .exe ([af175be](https://github.com/kodaskills/diffly/commit/af175beb083f4239f193a63db6116fdd2ccb108d))
- Fix auto change log tag already exists ([6dd80ee](https://github.com/kodaskills/diffly/commit/6dd80eeb2321dc47b9b8ddc03a3d1bba0f2fd750))
- Filter only relevant changed sources to run CI/CD workflows ([32f18c2](https://github.com/kodaskills/diffly/commit/32f18c21510d064268d78d88e96e4c442d62b2d0))
- Fix goreleaser ([b62ac37](https://github.com/kodaskills/diffly/commit/b62ac37713727e20125040471691dc132c495a28))
- Fix goreleaser ([cf008ab](https://github.com/kodaskills/diffly/commit/cf008abc963d5ada77eb22469690d4d6d76f3332))
- Fix goreleaser ([e306057](https://github.com/kodaskills/diffly/commit/e3060579098f4a36d2e7524e978b6dd1f7b2ac87))
- Remove goreleaser ([b968e94](https://github.com/kodaskills/diffly/commit/b968e94b05af1a07774a04d03aed62876060b3e1))


### 📝 Documentation
- Update readme for new 3 way merging feature ([de1b872](https://github.com/kodaskills/diffly/commit/de1b8725ec080bad877bab73909b3e8d40c7b4de))
- Update readme for new snapshot output path ([7d40a43](https://github.com/kodaskills/diffly/commit/7d40a43d5185ec04a72296f7ab2101f23d3427cb))
- Fix readme test command and typo ([97c0fe6](https://github.com/kodaskills/diffly/commit/97c0fe61baed2a50075dad98b6a7a7d416fe72ce))



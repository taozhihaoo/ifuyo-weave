# BRAND

## 品牌

- 产品名：**Weave**
- 出品方：**ifuyo（伊芙游）**
- 站点：<https://ifuyo.com>

品牌信息的单一事实源是根目录 `brand.json`；Rust（编译期嵌入）与前端（IPC `get_app_info`）都从它派生，禁止各写一份。

## 所有权 / Ownership

"ifuyo / Weave" 名称与品牌资产归 ifuyo 所有。第三方分发须保留名称与许可声明（MIT OR Apache-2.0 仅覆盖代码）。

## 资产策略 / Asset Policy

- M0 使用 `scripts/generate-icon.mjs` 生成的**工程占位图标**（accent 圆角方块 + 编织纹理），
  不是品牌资产；官方 logo（如确定）替换 `assets/icon-source.png` 后重跑
  `npm run generate:icon` 即可全量再生。
- 禁止把私有品牌资源（未公开的 logo 源文件等）随意塞进仓库。
- 视觉方向（charter #30/#31）：干净、克制、现代、中性、有一点 ifuyo 情绪；
  不复制 Aura 的音乐视觉，不做科技感仪表盘。token 单一来源见 `ui/src/design/tokens.ts`。

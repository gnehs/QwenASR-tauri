import { createElement as h } from "react";
import { mkdir, stat, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { render } from "takumi-js";

const WIDTH = 1280;
const HEIGHT = 640;
const MAX_FILE_SIZE = 1_000_000;

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outputPath = resolve(projectRoot, ".github/social-preview.png");

function AppIcon() {
  const barHeights = [70, 112, 160, 112, 70];

  return h(
    "div",
    {
      style: {
        position: "relative",
        zIndex: 2,
        width: 356,
        height: 356,
        marginLeft: 108,
        borderRadius: 88,
        background: "linear-gradient(145deg, #D8FBE7 0%, #9BE9C9 100%)",
        boxShadow:
          "0 30px 70px rgba(27,140,104,0.18), inset 0 1px 0 rgba(255,255,255,0.7)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      },
    },
    h(
      "div",
      { style: { display: "flex", alignItems: "center", gap: 21 } },
      ...barHeights.map((height, index) =>
        h("div", {
          key: index,
          style: {
            width: 30,
            height,
            borderRadius: 18,
            background: "#FFFFFF",
            boxShadow: "0 8px 18px rgba(27,140,104,0.12)",
          },
        }),
      ),
    ),
  );
}

function SocialPreview() {
  return h(
    "div",
    {
      style: {
        width: "100%",
        height: "100%",
        position: "relative",
        overflow: "hidden",
        display: "flex",
        alignItems: "center",
        background: "#FAFCFB",
        color: "#171A19",
        fontFamily: "sans-serif",
      },
    },
    h("div", {
      style: {
        position: "absolute",
        width: 560,
        height: 560,
        left: -185,
        top: 40,
        borderRadius: 280,
        background:
          "radial-gradient(circle, rgba(155,233,201,0.34) 0%, rgba(216,251,231,0) 70%)",
      },
    }),
    h("div", {
      style: {
        position: "absolute",
        width: 480,
        height: 480,
        right: -150,
        bottom: -180,
        borderRadius: 240,
        background:
          "radial-gradient(circle, rgba(216,251,231,0.55) 0%, rgba(216,251,231,0) 72%)",
      },
    }),
    h(
      "svg",
      {
        width: WIDTH,
        height: HEIGHT,
        viewBox: `0 0 ${WIDTH} ${HEIGHT}`,
        style: { position: "absolute", inset: 0, opacity: 0.5 },
      },
      h("path", {
        d: "M-80 470 C 80 340, 150 560, 310 430 S 540 330, 680 450 S 920 540, 1080 410 S 1260 330, 1360 410",
        fill: "none",
        stroke: "#B9F2D7",
        strokeWidth: 3,
      }),
      h("path", {
        d: "M-80 495 C 90 375, 155 585, 320 455 S 550 355, 690 475 S 930 565, 1090 435 S 1265 360, 1360 440",
        fill: "none",
        stroke: "#D8FBE7",
        strokeWidth: 2,
      }),
    ),
    h(AppIcon),
    h(
      "div",
      {
        style: {
          position: "relative",
          zIndex: 2,
          marginLeft: 92,
          width: 620,
          display: "flex",
          flexDirection: "column",
          alignItems: "flex-start",
        },
      },
      h(
        "div",
        {
          style: {
            display: "flex",
            alignItems: "center",
            gap: 12,
            marginBottom: 26,
            color: "#187757",
            fontSize: 22,
            fontWeight: 700,
            letterSpacing: "0.12em",
          },
        },
        h("span", {
          style: {
            width: 9,
            height: 9,
            borderRadius: 9,
            background: "#1B8C68",
          },
        }),
        "PRIVATE · ON-DEVICE",
      ),
      h(
        "h1",
        {
          style: {
            margin: 0,
            fontSize: 76,
            lineHeight: 1,
            fontWeight: 760,
            letterSpacing: "-0.045em",
            color: "#111312",
          },
        },
        "QwenASR Studio",
      ),
      h(
        "p",
        {
          style: {
            margin: "26px 0 0",
            fontSize: 32,
            lineHeight: 1.3,
            fontWeight: 500,
            letterSpacing: "-0.018em",
            color: "#4A514E",
          },
        },
        "Private, local speech-to-text for Mac",
      ),
      h(
        "div",
        {
          style: {
            display: "flex",
            alignItems: "center",
            marginTop: 34,
            gap: 16,
            fontSize: 22,
            fontWeight: 600,
            color: "#737B77",
          },
        },
        h("span", null, "Audio & video"),
        h("span", { style: { color: "#AAB2AE" } }, "·"),
        h("span", null, "Transcripts"),
        h("span", { style: { color: "#AAB2AE" } }, "·"),
        h("span", null, "SRT"),
      ),
    ),
  );
}

await mkdir(dirname(outputPath), { recursive: true });

const png = await render(h(SocialPreview), {
  width: WIDTH,
  height: HEIGHT,
});

await writeFile(outputPath, png);

const { size } = await stat(outputPath);

if (size >= MAX_FILE_SIZE) {
  throw new Error(
    `Generated image is ${size} bytes; GitHub requires files under ${MAX_FILE_SIZE} bytes.`,
  );
}

console.log(
  `Generated ${outputPath} (${WIDTH}x${HEIGHT}, ${(size / 1024).toFixed(1)} KiB)`,
);

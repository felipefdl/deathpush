import type { APIRoute } from "astro";
import satori from "satori";
import { Resvg } from "@resvg/resvg-js";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const interBold = readFileSync(resolve("src/assets/inter-bold.ttf"));
const logoSvg = readFileSync(resolve("public/favicon.svg"), "utf-8");
const logoDataUri = `data:image/svg+xml;base64,${Buffer.from(logoSvg).toString("base64")}`;

const watermarkSvg = readFileSync(resolve("../imagem/deathlogo.svg"), "utf-8")
  .replace('fill="currentColor"', 'fill="rgba(255,255,255,0.03)"');
const watermarkDataUri = `data:image/svg+xml;base64,${Buffer.from(watermarkSvg).toString("base64")}`;

export const GET: APIRoute = async () => {
  const svg = await satori(
    {
      type: "div",
      props: {
        style: {
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          backgroundColor: "#0a0a0a",
          padding: "60px",
          fontFamily: "Inter",
          position: "relative",
          overflow: "hidden",
        },
        children: [
          {
            type: "img",
            props: {
              src: watermarkDataUri,
              width: 800,
              height: 800,
              style: {
                position: "absolute",
                left: "-100px",
                top: "-80px",
              },
            },
          },
          {
            type: "div",
            props: {
              style: {
                display: "flex",
                flexDirection: "column",
                flex: 1,
                justifyContent: "center",
              },
              children: [
                {
                  type: "div",
                  props: {
                    style: {
                      display: "flex",
                      alignItems: "center",
                      gap: "16px",
                      marginBottom: "32px",
                    },
                    children: [
                      {
                        type: "img",
                        props: {
                          src: logoDataUri,
                          width: 48,
                          height: 48,
                          style: {
                            borderRadius: "10px",
                          },
                        },
                      },
                      {
                        type: "div",
                        props: {
                          style: {
                            fontSize: "28px",
                            fontWeight: 700,
                            color: "#f5f5f5",
                            letterSpacing: "-0.02em",
                          },
                          children: "DeathPush",
                        },
                      },
                    ],
                  },
                },
                {
                  type: "div",
                  props: {
                    style: {
                      fontSize: "56px",
                      fontWeight: 700,
                      color: "#f5f5f5",
                      lineHeight: 1.1,
                      letterSpacing: "-0.03em",
                      marginBottom: "20px",
                    },
                    children: "Murder the Noise. Ship the Code.",
                  },
                },
                {
                  type: "div",
                  props: {
                    style: {
                      fontSize: "22px",
                      color: "#999999",
                      lineHeight: 1.5,
                    },
                    children:
                      "A standalone desktop Git client with the VS Code Source Control UX. Beautiful diffs, clean staging, zero bloat.",
                  },
                },
              ],
            },
          },
          {
            type: "div",
            props: {
              style: {
                display: "flex",
                justifyContent: "flex-end",
                alignItems: "flex-end",
              },
              children: [
                {
                  type: "div",
                  props: {
                    style: {
                      display: "flex",
                      borderRadius: "12px",
                      border: "1px solid #222222",
                      overflow: "hidden",
                      width: "460px",
                      height: "200px",
                      backgroundColor: "#0d0d0d",
                      flexDirection: "row",
                    },
                    children: [
                      {
                        type: "div",
                        props: {
                          style: {
                            width: "140px",
                            height: "100%",
                            borderRight: "1px solid #222222",
                            padding: "12px",
                            display: "flex",
                            flexDirection: "column",
                            gap: "6px",
                          },
                          children: [
                            {
                              type: "div",
                              props: {
                                style: {
                                  fontSize: "8px",
                                  fontWeight: 700,
                                  color: "#666",
                                  letterSpacing: "0.05em",
                                  marginBottom: "2px",
                                },
                                children: "STAGED CHANGES",
                              },
                            },
                            ...["70%", "50%", "80%"].map((w) => ({
                              type: "div" as const,
                              props: {
                                style: {
                                  display: "flex",
                                  alignItems: "center",
                                  gap: "6px",
                                },
                                children: [
                                  {
                                    type: "div" as const,
                                    props: {
                                      style: {
                                        width: "6px",
                                        height: "6px",
                                        borderRadius: "50%",
                                        backgroundColor: "#28c840",
                                      },
                                    },
                                  },
                                  {
                                    type: "div" as const,
                                    props: {
                                      style: {
                                        width: w,
                                        height: "6px",
                                        borderRadius: "3px",
                                        backgroundColor: "#222",
                                      },
                                    },
                                  },
                                ],
                              },
                            })),
                            {
                              type: "div",
                              props: {
                                style: {
                                  fontSize: "8px",
                                  fontWeight: 700,
                                  color: "#666",
                                  letterSpacing: "0.05em",
                                  marginTop: "4px",
                                  marginBottom: "2px",
                                },
                                children: "CHANGES",
                              },
                            },
                            ...["55%", "75%"].map((w) => ({
                              type: "div" as const,
                              props: {
                                style: {
                                  display: "flex",
                                  alignItems: "center",
                                  gap: "6px",
                                },
                                children: [
                                  {
                                    type: "div" as const,
                                    props: {
                                      style: {
                                        width: "6px",
                                        height: "6px",
                                        borderRadius: "50%",
                                        backgroundColor: "#dc2626",
                                      },
                                    },
                                  },
                                  {
                                    type: "div" as const,
                                    props: {
                                      style: {
                                        width: w,
                                        height: "6px",
                                        borderRadius: "3px",
                                        backgroundColor: "#222",
                                      },
                                    },
                                  },
                                ],
                              },
                            })),
                          ],
                        },
                      },
                      {
                        type: "div",
                        props: {
                          style: {
                            flex: 1,
                            padding: "12px",
                            display: "flex",
                            flexDirection: "column",
                            gap: "3px",
                          },
                          children: [
                            ...[
                              { bg: "transparent", w: "90%" },
                              { bg: "transparent", w: "60%" },
                              { bg: "rgba(220, 38, 38, 0.15)", w: "75%" },
                              { bg: "rgba(34, 197, 94, 0.15)", w: "80%" },
                              { bg: "rgba(34, 197, 94, 0.15)", w: "65%" },
                              { bg: "transparent", w: "85%" },
                              { bg: "transparent", w: "40%" },
                              { bg: "rgba(34, 197, 94, 0.15)", w: "60%" },
                            ].map((line) => ({
                              type: "div" as const,
                              props: {
                                style: {
                                  display: "flex",
                                  alignItems: "center",
                                  padding: "3px 8px",
                                  backgroundColor: line.bg,
                                  borderRadius: "2px",
                                },
                                children: [
                                  {
                                    type: "div" as const,
                                    props: {
                                      style: {
                                        width: line.w,
                                        height: "5px",
                                        borderRadius: "2px",
                                        backgroundColor:
                                          line.bg === "transparent"
                                            ? "#1e1e1e"
                                            : line.bg.includes("38, 38")
                                              ? "rgba(220, 38, 38, 0.3)"
                                              : "rgba(34, 197, 94, 0.3)",
                                      },
                                    },
                                  },
                                ],
                              },
                            })),
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
    {
      width: 1200,
      height: 630,
      fonts: [
        {
          name: "Inter",
          data: interBold,
          weight: 700,
          style: "normal" as const,
        },
      ],
    }
  );

  const resvg = new Resvg(svg, {
    fitTo: { mode: "width", value: 1200 },
  });
  const pngData = resvg.render();
  const pngBuffer = pngData.asPng();

  return new Response(pngBuffer, {
    headers: {
      "Content-Type": "image/png",
      "Cache-Control": "public, max-age=31536000, immutable",
    },
  });
};

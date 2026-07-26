import type { Metadata } from "next";
import "@/app/globals.css";

export const metadata: Metadata = {
  title: "RID",
  description: "Bind apps to a Windows main application.",
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="zh-CN">
      <body>{children}</body>
    </html>
  );
}


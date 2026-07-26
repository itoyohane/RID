import Image from "next/image";

export function RidMark({
  size = "medium",
}: {
  size?: "small" | "medium" | "large";
}) {
  return (
    <span className={`rid-mark rid-mark--${size}`} aria-hidden="true">
      <Image
        src="/assets/rid-logo.svg"
        alt=""
        width={64}
        height={64}
        priority
      />
    </span>
  );
}

import { cn } from "cn"

// Geometry copied from docs/design/design-system/logos/tabwallet-logo.svg.
// Fills use theme variables so the signature follows the light and dark themes.
const WORDMARK_TAB =
  "M474 0V-1186H20V-1440H1200V-1186H746V0ZM1600.0 30Q1484.0 30 1403.5 -14.5Q1323.0 -59 1281.5 -133.5Q1240.0 -208 1240.0 -298Q1240.0 -373 1263.0 -435.0Q1286.0 -497 1337.5 -544.5Q1389.0 -592 1476.0 -624Q1536.0 -646 1619.0 -663.0Q1702.0 -680 1807.0 -695.5Q1912.0 -711 2038.0 -730L1940.0 -676Q1940.0 -772 1894.0 -817.0Q1848.0 -862 1740.0 -862Q1680.0 -862 1615.0 -833.0Q1550.0 -804 1524.0 -730L1278.0 -808Q1319.0 -942 1432.0 -1026.0Q1545.0 -1110 1740.0 -1110Q1883.0 -1110 1994.0 -1066.0Q2105.0 -1022 2162.0 -914Q2194.0 -854 2200.0 -794.0Q2206.0 -734 2206.0 -660V0H1968.0V-222L2002.0 -176Q1923.0 -67 1831.5 -18.5Q1740.0 30 1600.0 30ZM1658.0 -184Q1733.0 -184 1784.5 -210.5Q1836.0 -237 1866.5 -271.0Q1897.0 -305 1908.0 -328Q1929.0 -372 1932.5 -430.5Q1936.0 -489 1936.0 -528L2016.0 -508Q1895.0 -488 1820.0 -474.5Q1745.0 -461 1699.0 -450.0Q1653.0 -439 1618.0 -426Q1578.0 -410 1553.5 -391.5Q1529.0 -373 1517.5 -351.0Q1506.0 -329 1506.0 -302Q1506.0 -265 1524.5 -238.5Q1543.0 -212 1577.0 -198.0Q1611.0 -184 1658.0 -184ZM2936.0 30Q2775.0 30 2667.0 -45.0Q2559.0 -120 2504.5 -249.0Q2450.0 -378 2450.0 -540Q2450.0 -702 2504.0 -831.0Q2558.0 -960 2663.0 -1035.0Q2768.0 -1110 2922.0 -1110Q3077.0 -1110 3192.0 -1036.0Q3307.0 -962 3370.5 -833.5Q3434.0 -705 3434.0 -540Q3434.0 -378 3371.5 -249.0Q3309.0 -120 3197.0 -45.0Q3085.0 30 2936.0 30ZM2404.0 0V-1440H2678.0V-740H2644.0V0ZM2892.0 -212Q2980.0 -212 3036.0 -256.0Q3092.0 -300 3119.0 -374.5Q3146.0 -449 3146.0 -540Q3146.0 -630 3118.0 -704.5Q3090.0 -779 3031.5 -823.5Q2973.0 -868 2882.0 -868Q2797.0 -868 2744.5 -827.0Q2692.0 -786 2668.0 -712.0Q2644.0 -638 2644.0 -540Q2644.0 -442 2668.0 -368.0Q2692.0 -294 2746.5 -253.0Q2801.0 -212 2892.0 -212Z"
const WORDMARK_WALLET =
  "M3896.0 0 3474.0 -1440H3760.0L4030.0 -450L4300.0 -1438L4586.0 -1440L4856.0 -450L5126.0 -1440H5412.0L4990.0 0H4722.0L4443.0 -972L4164.0 0ZM5812.0 30Q5696.0 30 5615.5 -14.5Q5535.0 -59 5493.5 -133.5Q5452.0 -208 5452.0 -298Q5452.0 -373 5475.0 -435.0Q5498.0 -497 5549.5 -544.5Q5601.0 -592 5688.0 -624Q5748.0 -646 5831.0 -663.0Q5914.0 -680 6019.0 -695.5Q6124.0 -711 6250.0 -730L6152.0 -676Q6152.0 -772 6106.0 -817.0Q6060.0 -862 5952.0 -862Q5892.0 -862 5827.0 -833.0Q5762.0 -804 5736.0 -730L5490.0 -808Q5531.0 -942 5644.0 -1026.0Q5757.0 -1110 5952.0 -1110Q6095.0 -1110 6206.0 -1066.0Q6317.0 -1022 6374.0 -914Q6406.0 -854 6412.0 -794.0Q6418.0 -734 6418.0 -660V0H6180.0V-222L6214.0 -176Q6135.0 -67 6043.5 -18.5Q5952.0 30 5812.0 30ZM5870.0 -184Q5945.0 -184 5996.5 -210.5Q6048.0 -237 6078.5 -271.0Q6109.0 -305 6120.0 -328Q6141.0 -372 6144.5 -430.5Q6148.0 -489 6148.0 -528L6228.0 -508Q6107.0 -488 6032.0 -474.5Q5957.0 -461 5911.0 -450.0Q5865.0 -439 5830.0 -426Q5790.0 -410 5765.5 -391.5Q5741.0 -373 5729.5 -351.0Q5718.0 -329 5718.0 -302Q5718.0 -265 5736.5 -238.5Q5755.0 -212 5789.0 -198.0Q5823.0 -184 5870.0 -184ZM6638.0 0V-1470H6910.0V0ZM7170.0 0V-1470H7442.0V0ZM8184.0 30Q8018.0 30 7891.5 -41.5Q7765.0 -113 7693.5 -238.5Q7622.0 -364 7622.0 -526Q7622.0 -703 7692.0 -834.0Q7762.0 -965 7885.0 -1037.5Q8008.0 -1110 8168.0 -1110Q8338.0 -1110 8457.0 -1030.0Q8576.0 -950 8633.0 -805.0Q8690.0 -660 8673.0 -464H8404.0V-564Q8404.0 -729 8351.5 -801.5Q8299.0 -874 8180.0 -874Q8041.0 -874 7975.5 -789.5Q7910.0 -705 7910.0 -540Q7910.0 -389 7975.5 -306.5Q8041.0 -224 8168.0 -224Q8248.0 -224 8305.0 -259.0Q8362.0 -294 8392.0 -360L8664.0 -282Q8603.0 -134 8471.5 -52.0Q8340.0 30 8184.0 30ZM7826.0 -464V-666H8542.0V-464ZM9486.0 0Q9374.0 21 9266.5 18.5Q9159.0 16 9074.5 -19.5Q8990.0 -55 8946.0 -134Q8906.0 -208 8904.0 -284.5Q8902.0 -361 8902.0 -458V-1380H9174.0V-474Q9174.0 -411 9175.5 -360.5Q9177.0 -310 9196.0 -280Q9232.0 -223 9311.0 -218.0Q9390.0 -213 9486.0 -226ZM8718.0 -870V-1080H9486.0V-870Z"
const WORDMARK_TRANSFORM = "translate(77.49 51.43) scale(0.0256)"

const TONES = {
  default: {
    tab: "fill-logo-tab",
    body: "fill-primary",
    clasp: "fill-logo-clasp",
    wordTab: "fill-primary",
    wordWallet: "fill-foreground",
  },
  white: {
    tab: "fill-white",
    body: "fill-white",
    clasp: "fill-primary",
    wordTab: "fill-white",
    wordWallet: "fill-white",
  },
  mono: {
    tab: "fill-foreground",
    body: "fill-foreground",
    clasp: "fill-card",
    wordTab: "fill-foreground",
    wordWallet: "fill-foreground",
  },
} as const

const FULL_WIDTH = 321
const MARK_WIDTH = 64
const HEIGHT = 64

export interface LogoProps {
  variant?: "full" | "mark"
  /** Rendered height in px; the width follows the aspect ratio. */
  height?: number
  /** `white` only on a brand-primary fill in the light theme. */
  tone?: "default" | "white" | "mono"
  /** Accessible name. */
  label?: string
  className?: string
}

/** The TabWallet mark or full signature, coloured by theme tokens. */
export function Logo({
  variant = "full",
  height = 32,
  tone = "default",
  label = "TabWallet",
  className,
}: LogoProps) {
  const viewWidth = variant === "mark" ? MARK_WIDTH : FULL_WIDTH
  const width = (height * viewWidth) / HEIGHT

  const { tab, body, clasp, wordTab, wordWallet } = TONES[tone]

  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox={`0 0 ${viewWidth} ${HEIGHT}`}
      width={width}
      height={height}
      role="img"
      aria-label={label}
      className={cn("shrink-0", className)}
    >
      <rect className={tab} x="8" y="8" width="26" height="16" rx="6" />
      <rect className={body} x="4" y="16" width="56" height="42" rx="11" />
      <rect className={tab} x="38" y="30" width="22" height="14" rx="7" />
      <circle className={clasp} cx="46" cy="37" r="3" />
      {variant === "full" && (
        <g transform={WORDMARK_TRANSFORM}>
          <path className={wordTab} d={WORDMARK_TAB} />
          <path className={wordWallet} d={WORDMARK_WALLET} />
        </g>
      )}
    </svg>
  )
}

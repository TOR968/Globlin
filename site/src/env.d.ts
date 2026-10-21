interface NavigatorUAData {
  readonly platform: string;
  getHighEntropyValues(hints: string[]): Promise<{ architecture?: string }>;
}

interface Navigator {
  readonly userAgentData?: NavigatorUAData;
}

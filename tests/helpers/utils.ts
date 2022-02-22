export const getUnixTimestamp = () => {
  return Math.floor(new Date().getTime() / 1000);
};

export async function sleep(seconds): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, seconds * 1000));
}

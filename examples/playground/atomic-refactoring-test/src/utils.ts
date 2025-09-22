export class StringUtils {
  static capitalize(text: string): string {
    return text.charAt(0).toUpperCase() + text.slice(1);
  }

  static reverse(text: string): string {
    return text.split('').reverse().join('');
  }
}
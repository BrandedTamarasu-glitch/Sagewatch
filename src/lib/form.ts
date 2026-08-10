export function checkedSetting(form: HTMLFormElement, name: string): boolean {
  return form.querySelector<HTMLInputElement>(`input[name="${name}"]`)?.checked === true;
}

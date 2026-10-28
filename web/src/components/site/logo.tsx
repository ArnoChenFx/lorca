export function Logo({ className = 'size-7' }: { className?: string }) {
  // Exported from the app icon.
  return (
    <img src="/icon.png" width={256} height={256} className={className} alt="" aria-hidden="true" />
  )
}

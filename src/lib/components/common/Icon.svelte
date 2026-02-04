<script lang="ts">
  interface Props {
    name: string;
    filled?: boolean;
    size?: "sm" | "md" | "lg" | "xl";
    class?: string;
  }

  let { name, filled = false, size = "md", class: className = "" }: Props = $props();

  const sizeClasses = {
    sm: "text-[16px]",
    md: "text-[20px]",
    lg: "text-[24px]",
    xl: "text-[32px]",
  };
</script>

<span
  class="material-symbols-outlined {sizeClasses[size]} {filled ? 'filled' : ''} {className}"
>
  {name}
</span>

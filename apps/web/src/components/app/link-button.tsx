'use client';

import Link from 'next/link';
import type {VariantProps} from 'class-variance-authority';
import {buttonVariants} from '@/components/lime/ui/button';

type Props = React.ComponentProps<typeof Link> & VariantProps<typeof buttonVariants> & {
  /** A plain <a> for downloads and routes that are not pages. */
  external?: boolean;
};

/** A link that looks like a Lime button, for navigation (a Button is for actions). */
export function LinkButton({variant, size, className, external, href, ...props}: Props) {
  const classes = buttonVariants({variant, size, className});
  if (external) return <a href={String(href)} className={classes} {...(props as React.ComponentProps<'a'>)}/>;
  return <Link href={href} className={classes} {...props}/>;
}

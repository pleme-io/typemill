export interface NavItem {
	label: string;
	href: string;
	children?: NavItem[];
}

/**
 * Navigation structure matching actual docs
 */
export function generateNavigation(): NavItem[] {
	return [
		{
			label: 'Home',
			href: '/'
		},
		{
			label: 'User Guide',
			href: '/user-guide/getting-started',
			children: [
				{ label: 'Getting Started', href: '/user-guide/getting-started' },
				{ label: 'Configuration', href: '/user-guide/configuration' },
				{ label: 'Cheatsheet', href: '/user-guide/cheatsheet' },
				{ label: 'Cookbook', href: '/user-guide/cookbook' },
				{ label: 'Troubleshooting', href: '/user-guide/troubleshooting' }
			]
		},
		{
			label: 'Tools',
			href: '/tools',
			children: [
				{ label: 'Reference', href: '/tools' },
				{ label: 'refactor', href: '/tools/refactor' },
				{ label: 'workspace', href: '/tools/workspace' },
				{ label: 'system', href: '/tools/system' }
			]
		},
		{
			label: 'Architecture',
			href: '/architecture',
			children: [
				{ label: 'Core Concepts', href: '/architecture/core-concepts' },
				{ label: 'Specifications', href: '/architecture/specifications' },
				{ label: 'Public API', href: '/architecture/public_api' }
			]
		},
		{
			label: 'Development',
			href: '/development/overview',
			children: [
				{ label: 'Overview', href: '/development/overview' },
				{ label: 'Testing', href: '/development/testing' },
				{ label: 'Logging', href: '/development/logging_guidelines' },
				{ label: 'Plugins', href: '/development/plugin-development' }
			]
		},
		{
			label: 'Operations',
			href: '/operations/cache_configuration',
			children: [
				{ label: 'Cache Config', href: '/operations/cache_configuration' },
				{ label: 'CI/CD', href: '/operations/cicd' }
			]
		},
		{
			label: 'Contributing',
			href: '/contributing'
		}
	];
}

export function getSectionNav(section: string): NavItem[] {
	const nav = generateNavigation();
	const sectionItem = nav.find((item) => item.href.includes(section));
	return sectionItem?.children || [];
}

export type DocsLink = NavItem;
export const getAllDocsLinks = generateNavigation;

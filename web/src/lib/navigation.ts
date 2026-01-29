export interface NavItem {
	label: string;
	href: string;
	children?: NavItem[];
}

/**
 * Generate navigation structure from docs directory
 */
export function generateNavigation(): NavItem[] {
	return [
		{
			label: 'Home',
			href: '/'
		},
		{
			label: 'Documentation',
			href: '/docs',
			children: [
				{
					label: 'Getting Started',
					href: '/docs'
				},
				{
					label: 'Development Guide',
					href: '/DEVELOPMENT'
				}
			]
		},
		{
			label: 'Tools Reference',
			href: '/tools',
			children: [
				{
					label: 'Overview',
					href: '/tools'
				},
				{
					label: 'Navigation',
					href: '/tools/navigation'
				},
				{
					label: 'Refactoring',
					href: '/tools/refactor'
				},
				{
					label: 'Analysis',
					href: '/tools/analysis'
				},
				{
					label: 'System',
					href: '/tools/system'
				},
				{
					label: 'Workspace',
					href: '/tools/workspace'
				}
			]
		},
		{
			label: 'Architecture',
			href: '/architecture',
			children: [
				{
					label: 'Overview',
					href: '/architecture/overview'
				},
				{
					label: 'API Contracts',
					href: '/architecture/api_contracts'
				},
				{
					label: 'Layer Architecture',
					href: '/architecture/layers'
				},
				{
					label: 'Language Common API',
					href: '/architecture/lang_common_api'
				}
			]
		},
		{
			label: 'Operations',
			href: '/operations',
			children: [
				{
					label: 'Docker Deployment',
					href: '/operations/docker_deployment'
				},
				{
					label: 'Cache Configuration',
					href: '/operations/cache_configuration'
				},
				{
					label: 'CI/CD',
					href: '/operations/cicd'
				}
			]
		},
		{
			label: 'Development',
			href: '/development',
			children: [
				{
					label: 'Testing Guide',
					href: '/development/testing'
				},
				{
					label: 'Logging Guidelines',
					href: '/development/logging_guidelines'
				}
			]
		},
		{
			label: 'Contributing',
			href: '/contributing'
		}
	];
}

/**
 * Get navigation items for a specific section
 */
export function getSectionNav(section: string): NavItem[] {
	const nav = generateNavigation();
	const sectionItem = nav.find(item => item.href.includes(section));
	return sectionItem?.children || [];
}

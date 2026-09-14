export interface DocumentationNavigationItem {
	href: string;
	id: string;
	label: string;
}

export interface DocumentationNavigationSection {
	items: readonly DocumentationNavigationItem[];
	label: string;
}

export const documentationOverview = {
	href: "/docs/",
	id: "index",
	label: "Overview",
} as const satisfies DocumentationNavigationItem;

export const documentationNavigation = [
	{
		label: "Using Backbeat",
		items: [
			{
				href: "/docs/devs/what-is-backbeat/",
				id: "devs/what-is-backbeat",
				label: "What is Backbeat?",
			},
			{
				href: "/docs/devs/how-it-works",
				id: "devs/how-it-works",
				label: "How does Backbeat work?",
			},
			{
				href: "/docs/devs/integrate-backbeat/",
				id: "devs/integrate-backbeat",
				label: "Integrate Backbeat into your game",
			},
			{
				href: "/docs/devs/running-a-server/",
				id: "devs/running-a-server",
				label: "Run a Backbeat server",
			},
		],
	},
	{
		label: "Specifications",
		items: [
			{
				href: "/docs/specs/backbeat-file/",
				id: "specs/backbeat-file",
				label: "Backbeat files",
			},
			{
				href: "/docs/specs/backbeat-zip/",
				id: "specs/backbeat-zip",
				label: "Backbeat zip files",
			},
			{
				href: "/docs/specs/backbeat-table/",
				id: "specs/backbeat-table",
				label: "Tables",
			},
			{
				href: "/docs/specs/backbeat-course/",
				id: "specs/backbeat-course",
				label: "Courses",
			},
			{
				href: "/docs/specs/backbeat-pack/",
				id: "specs/backbeat-pack",
				label: "Packs",
			},
			{
				href: "/docs/specs/bundle-id/",
				id: "specs/bundle-id",
				label: "Bundle IDs",
			},
			{
				href: "/docs/specs/backbeat-store/",
				id: "specs/backbeat-store",
				label: "The Backbeat store",
			},
			{
				href: "/docs/specs/backbeat-config/",
				id: "specs/backbeat-config",
				label: "Backbeat configuration",
			},
			{
				href: "/docs/specs/backbeat-server/",
				id: "specs/backbeat-server",
				label: "Data servers",
			},
			{
				href: "/docs/specs/backbeat-collection-endpoint/",
				id: "specs/backbeat-collection-endpoint",
				label: "Collection endpoints",
			},
			{
				href: "/docs/specs/combined-assets/",
				id: "specs/combined-assets",
				label: "Combined assets",
			},
			{
				href: "/docs/specs/combined-assets-id/",
				id: "specs/combined-assets-id",
				label: "Combined asset IDs",
			},
		],
	},
] as const satisfies readonly DocumentationNavigationSection[];

export const documentationPages: readonly DocumentationNavigationItem[] = [
	documentationOverview,
	...documentationNavigation.flatMap((section) => section.items),
];

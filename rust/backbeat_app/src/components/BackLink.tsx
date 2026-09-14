import { useNavigate } from "@solidjs/router";

export function BackLink() {
	const navigate = useNavigate();

	return (
		<a
			href="#back"
			onClick={(event) => {
				event.preventDefault();
				navigate(-1);
			}}
		>
			&larr; Back
		</a>
	);
}

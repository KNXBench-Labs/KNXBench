// Build synthetic expanded-name ownership cases without changing static product semantics.
fn namespace_cases(xml: &str) -> Vec<(&'static str, String)> {
    let foreign = xml.replace(
        "<KNX xmlns=",
        "<KNX xmlns:e=\"urn:synthetic:foreign\" xmlns=",
    );
    let inactive = "<ParameterRefRef RefId=\"M-FFF6_A-0001-01-0000_P-I_R-I\" />";
    vec![
        (
            "root-inherited-empty-ref",
            foreign.replace(inactive, &inactive.replacen('<', "<e:", 1)),
        ),
        (
            "application-inherited-open-ref",
            xml.replace(
                "<ApplicationProgram Id=",
                "<ApplicationProgram xmlns:e=\"urn:synthetic:foreign\" Id=",
            )
            .replace(
                inactive,
                "<e:ParameterRefRef RefId=\"M-FFF6_A-0001-01-0000_P-I_R-I\"></e:ParameterRefRef>",
            ),
        ),
        (
            "foreign-choose",
            foreign
                .replace("<choose ", "<e:choose ")
                .replace("</choose>", "</e:choose>"),
        ),
        (
            "foreign-when",
            foreign
                .replace("<when ", "<e:when ")
                .replace("</when>", "</e:when>"),
        ),
        (
            "foreign-wrapper",
            foreign
                .replace("<ParameterBlock ", "<e:ParameterBlock ")
                .replace("</ParameterBlock>", "</e:ParameterBlock>"),
        ),
        (
            "unbound-prefix",
            xml.replace(inactive, &inactive.replacen('<', "<e:", 1)),
        ),
        (
            "foreign-application-default",
            xml.replace(
                "<ApplicationProgram Id=",
                "<ApplicationProgram xmlns=\"urn:synthetic:foreign\" Id=",
            ),
        ),
        (
            "dynamic-default-reset",
            xml.replace("<Dynamic>", "<Dynamic xmlns=\"\">"),
        ),
        (
            "dynamic-prefix-rebinding",
            foreign
                .replace("<Dynamic>", "<Dynamic xmlns:e=\"urn:synthetic:rebound\">")
                .replace(inactive, &inactive.replacen('<', "<e:", 1)),
        ),
    ]
}

fn namespace_positive(xml: &str, application_scope: bool) -> String {
    let xml = if application_scope {
        xml.replace(
            "<ApplicationProgram Id=",
            "<ApplicationProgram xmlns:e=\"http://knx.org/xml/project/20\" Id=",
        )
    } else {
        xml.replace(
            "<KNX xmlns=",
            "<KNX xmlns:e=\"http://knx.org/xml/project/20\" xmlns=",
        )
    };
    xml.replace("<ParameterRefRef ", "<e:ParameterRefRef ")
        .replace("<choose ", "<e:choose ")
        .replace("</choose>", "</e:choose>")
        .replace("<when ", "<e:when ")
        .replace("</when>", "</e:when>")
        .replace("<ParameterBlock ", "<e:ParameterBlock ")
        .replace("</ParameterBlock>", "</e:ParameterBlock>")
}
